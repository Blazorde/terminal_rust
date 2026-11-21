use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::manifest::MANIFEST_FILE;
use super::{Compatibility, PluginManifest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginStatus {
    Enabled,
    Disabled,
    Incompatible,
    /// A newer compatible version of the same plugin is installed and used instead.
    Superseded,
}

#[derive(Debug)]
pub(crate) enum RegistryEntry {
    Installed {
        compatibility: Compatibility,
        manifest: Box<PluginManifest>,
        path: PathBuf,
        status: PluginStatus,
    },
    Invalid {
        path: PathBuf,
        message: String,
    },
}

#[derive(Debug)]
pub(crate) struct RegistryError {
    path: PathBuf,
    source: io::Error,
}

pub(crate) fn discover_default(enabled: &[String]) -> Result<Vec<RegistryEntry>, RegistryError> {
    let Some(root) = default_plugin_path() else {
        return Ok(Vec::new());
    };
    discover(&root, enabled)
}

pub(crate) fn discover(
    root: &Path,
    enabled: &[String],
) -> Result<Vec<RegistryEntry>, RegistryError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut manifests = Vec::new();
    let mut unreadable = Vec::new();
    collect_manifest_paths(root, 0, &mut manifests, &mut unreadable)?;
    manifests.sort();

    let mut entries = manifests
        .into_iter()
        .map(|path| match PluginManifest::load(&path) {
            Ok(manifest) if !directory_matches_id(root, &path, &manifest.id) => {
                RegistryEntry::Invalid {
                    message: format!(
                        "plugin id '{}' does not match its directory name",
                        manifest.id
                    ),
                    path,
                }
            }
            Ok(manifest) => {
                let compatibility = manifest.compatibility();
                let status = if !compatibility.is_compatible() {
                    PluginStatus::Incompatible
                } else if enabled.iter().any(|id| id == &manifest.id) {
                    PluginStatus::Enabled
                } else {
                    PluginStatus::Disabled
                };
                RegistryEntry::Installed {
                    compatibility,
                    manifest: Box::new(manifest),
                    path,
                    status,
                }
            }
            Err(error) => RegistryEntry::Invalid {
                path,
                message: error.to_string(),
            },
        })
        .collect::<Vec<_>>();

    // A folder that cannot be read is reported, it does not hide the other plugins.
    entries.extend(
        unreadable
            .into_iter()
            .map(|(path, message)| RegistryEntry::Invalid { path, message }),
    );
    supersede_older_versions(&mut entries);
    Ok(entries)
}

/// The layout is `<root>/<id>/<version>/terminal-rust-plugin.toml`. The grants
/// and the enabled list are keyed by the id, so it must be the folder's name.
fn directory_matches_id(root: &Path, manifest_path: &Path, id: &str) -> bool {
    let components = manifest_path
        .strip_prefix(root)
        .map(|relative| relative.components().collect::<Vec<_>>())
        .unwrap_or_default();
    match components.as_slice() {
        [first, _version, _file] => first.as_os_str() == id,
        _ => true,
    }
}

/// Two installed versions of one plugin declare the same commands, which would
/// block every one of them as a conflict. Only the newest compatible version stays
/// in use; the others are reported as superseded.
fn supersede_older_versions(entries: &mut [RegistryEntry]) {
    let mut newest: std::collections::HashMap<String, semver::Version> =
        std::collections::HashMap::new();
    for entry in entries.iter() {
        if let RegistryEntry::Installed {
            manifest,
            compatibility,
            ..
        } = entry
            && compatibility.is_compatible()
        {
            newest
                .entry(manifest.id.clone())
                .and_modify(|best| {
                    if manifest.version > *best {
                        best.clone_from(&manifest.version);
                    }
                })
                .or_insert_with(|| manifest.version.clone());
        }
    }
    for entry in entries.iter_mut() {
        if let RegistryEntry::Installed {
            manifest,
            compatibility,
            status,
            ..
        } = entry
            && compatibility.is_compatible()
            && newest
                .get(&manifest.id)
                .is_some_and(|best| manifest.version < *best)
        {
            *status = PluginStatus::Superseded;
        }
    }
}

/// Collect the manifests below `directory`. The root must be readable; anything
/// deeper that cannot be read (permissions, a folder removed meanwhile) is
/// recorded in `unreadable` and skipped.
fn collect_manifest_paths(
    directory: &Path,
    depth: usize,
    manifests: &mut Vec<PathBuf>,
    unreadable: &mut Vec<(PathBuf, String)>,
) -> Result<(), RegistryError> {
    if depth > 2 {
        return Ok(());
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(source) if depth > 0 => {
            unreadable.push((directory.to_path_buf(), source.to_string()));
            return Ok(());
        }
        Err(source) => {
            return Err(RegistryError {
                path: directory.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(source) => {
                unreadable.push((directory.to_path_buf(), source.to_string()));
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(source) => {
                unreadable.push((entry.path(), source.to_string()));
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_manifest_paths(&entry.path(), depth + 1, manifests, unreadable)?;
        } else if entry.file_name() == MANIFEST_FILE {
            manifests.push(entry.path());
        }
    }
    Ok(())
}

pub(crate) fn default_plugin_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|path| path.join("terminal-rust").join("plugins"))
    }
    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|path| {
                path.join("Library")
                    .join("Application Support")
                    .join("terminal-rust")
                    .join("plugins")
            })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_DATA_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .map(|path| path.join(".local").join("share"))
            })
            .map(|path| path.join("terminal-rust").join("plugins"))
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot inspect plugin directory {}: {}",
            self.path.display(),
            self.source
        )
    }
}

impl Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_directory_is_an_empty_registry() {
        let root = env::temp_dir().join("terminal-rust-definitely-missing-registry");
        assert!(discover(&root, &[]).unwrap().is_empty());
    }

    #[test]
    fn discovers_enabled_disabled_incompatible_and_invalid_plugins() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("terminal-rust-registry-{nonce}"));
        for (id, version, host_api, terminal_version) in [
            ("one", "1.0.0", "^1", ">=0.3, <2.0"),
            ("two", "2.0.0", "^1", ">=0.3, <2.0"),
            ("future", "1.0.0", "^2", ">=0.3, <2.0"),
            ("future-terminal", "1.0.0", "^1", ">=9"),
        ] {
            let directory = root.join(id).join(version);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join(MANIFEST_FILE), format!("schema_version = 1\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\ndescription = \"fixture\"\nhost_api = \"{host_api}\"\nterminal_version = \"{terminal_version}\"\n")).unwrap();
        }
        let invalid = root.join("broken").join("1.0.0");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(invalid.join(MANIFEST_FILE), "not = [valid").unwrap();

        let entries = discover(&root, &["one".into()]).unwrap();
        assert_eq!(entries.len(), 5);
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Enabled, compatibility: Compatibility::Compatible, .. } if manifest.id == "one")));
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Disabled, .. } if manifest.id == "two")));
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Incompatible, .. } if manifest.id == "future")));
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Incompatible, compatibility: Compatibility::TerminalMismatch { .. }, .. } if manifest.id == "future-terminal")));
        assert!(
            entries
                .iter()
                .any(|entry| matches!(entry, RegistryEntry::Invalid { .. }))
        );
    }

    fn manifest_text(id: &str, version: &str, terminal_version: &str) -> String {
        format!(
            "schema_version = 1\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\ndescription = \"fixture\"\nhost_api = \"^1\"\nterminal_version = \"{terminal_version}\"\npermissions = [\"command.register\", \"output.write\"]\n\n[[commands]]\nname = \"greet\"\nsummary = \"greet\"\nresponse = \"hello {version}\\n\"\n"
        )
    }

    fn install(root: &Path, directory: &str, id: &str, version: &str, terminal_version: &str) {
        let folder = root.join(directory).join(version);
        fs::create_dir_all(&folder).unwrap();
        fs::write(
            folder.join(MANIFEST_FILE),
            manifest_text(id, version, terminal_version),
        )
        .unwrap();
    }

    fn fresh_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("terminal-rust-registry-{name}-{nonce}"))
    }

    fn status_of(entries: &[RegistryEntry], wanted: &str) -> Option<PluginStatus> {
        entries.iter().find_map(|entry| match entry {
            RegistryEntry::Installed {
                manifest, status, ..
            } if manifest.version.to_string() == wanted => Some(*status),
            _ => None,
        })
    }

    #[test]
    fn only_the_newest_installed_version_of_a_plugin_stays_in_use() {
        let root = fresh_root("newest");
        install(&root, "tool", "tool", "1.0.0", ">=0.3, <2.0");
        install(&root, "tool", "tool", "1.2.0", ">=0.3, <2.0");
        install(&root, "tool", "tool", "1.10.0", ">=0.3, <2.0");

        let entries = discover(&root, &["tool".into()]).unwrap();

        assert_eq!(status_of(&entries, "1.10.0"), Some(PluginStatus::Enabled));
        assert_eq!(status_of(&entries, "1.2.0"), Some(PluginStatus::Superseded));
        assert_eq!(status_of(&entries, "1.0.0"), Some(PluginStatus::Superseded));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_older_compatible_version_is_kept_when_the_newest_is_incompatible() {
        let root = fresh_root("fallback");
        install(&root, "tool", "tool", "1.0.0", ">=0.3, <2.0");
        install(&root, "tool", "tool", "2.0.0", ">=99");

        let entries = discover(&root, &["tool".into()]).unwrap();

        assert_eq!(status_of(&entries, "1.0.0"), Some(PluginStatus::Enabled));
        assert_eq!(
            status_of(&entries, "2.0.0"),
            Some(PluginStatus::Incompatible)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn two_installed_versions_do_not_block_the_commands_of_the_plugin() {
        let root = fresh_root("conflict");
        install(&root, "tool", "tool", "1.0.0", ">=0.3, <2.0");
        install(&root, "tool", "tool", "1.1.0", ">=0.3, <2.0");
        let entries = discover(&root, &["tool".into()]).unwrap();
        let grants = std::collections::BTreeMap::from([(
            "tool".to_string(),
            vec!["command.register".to_string(), "output.write".to_string()],
        )]);

        let registry = crate::plugin::declarative::DeclarativeCommandRegistry::from_entries(
            &entries,
            &grants,
            Vec::new(),
        );

        assert_eq!(registry.blocked_reason("greet"), None);
        assert_eq!(
            registry
                .get("greet")
                .map(|command| command.response.as_str()),
            Some("hello 1.1.0\n")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_plugin_whose_id_differs_from_its_directory_is_invalid() {
        let root = fresh_root("id-mismatch");
        install(&root, "alpha", "beta", "1.0.0", ">=0.3, <2.0");

        let entries = discover(&root, &["beta".into()]).unwrap();

        assert_eq!(entries.len(), 1);
        assert!(matches!(
            &entries[0],
            RegistryEntry::Invalid { message, .. } if message.contains("does not match")
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_folder_is_reported_without_hiding_the_other_plugins() {
        use std::os::unix::fs::PermissionsExt;

        let root = fresh_root("unreadable");
        install(&root, "good", "good", "1.0.0", ">=0.3, <2.0");
        let locked = root.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        // Root ignores permissions: the folder is readable and there is nothing to test.
        let enforced = fs::read_dir(&locked).is_err();

        let entries = discover(&root, &["good".into()]);

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if enforced {
            let entries = entries.expect("an unreadable subfolder must not fail the discovery");
            assert_eq!(status_of(&entries, "1.0.0"), Some(PluginStatus::Enabled));
            assert!(entries.iter().any(|entry| matches!(
                entry,
                RegistryEntry::Invalid { path, .. } if path.ends_with("locked")
            )));
        }
        fs::remove_dir_all(root).unwrap();
    }
}
