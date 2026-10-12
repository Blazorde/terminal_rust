use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use semver::Version;

use super::PluginManifest;
use super::manifest::MANIFEST_FILE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PluginStatus {
    Enabled,
    Disabled,
    Incompatible,
}

#[derive(Debug)]
pub(crate) enum RegistryEntry {
    Installed {
        manifest: PluginManifest,
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
    collect_manifest_paths(root, 0, &mut manifests)?;
    manifests.sort();

    Ok(manifests
        .into_iter()
        .map(|path| match PluginManifest::load(&path) {
            Ok(manifest) => {
                let status = if !manifest.host_api.matches(&Version::new(1, 0, 0)) {
                    PluginStatus::Incompatible
                } else if enabled.iter().any(|id| id == &manifest.id) {
                    PluginStatus::Enabled
                } else {
                    PluginStatus::Disabled
                };
                RegistryEntry::Installed {
                    manifest,
                    path,
                    status,
                }
            }
            Err(error) => RegistryEntry::Invalid {
                path,
                message: error.to_string(),
            },
        })
        .collect())
}

fn collect_manifest_paths(
    directory: &Path,
    depth: usize,
    manifests: &mut Vec<PathBuf>,
) -> Result<(), RegistryError> {
    if depth > 2 {
        return Ok(());
    }
    let entries = fs::read_dir(directory).map_err(|source| RegistryError {
        path: directory.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| RegistryError {
            path: directory.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| RegistryError {
            path: entry.path(),
            source,
        })?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_manifest_paths(&entry.path(), depth + 1, manifests)?;
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
        for (id, version, host_api) in [
            ("one", "1.0.0", "^1"),
            ("two", "2.0.0", "^1"),
            ("future", "1.0.0", "^2"),
        ] {
            let directory = root.join(id).join(version);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join(MANIFEST_FILE), format!("schema_version = 1\nid = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\ndescription = \"fixture\"\nhost_api = \"{host_api}\"\n")).unwrap();
        }
        let invalid = root.join("broken").join("1.0.0");
        fs::create_dir_all(&invalid).unwrap();
        fs::write(invalid.join(MANIFEST_FILE), "not = [valid").unwrap();

        let entries = discover(&root, &["one".into()]).unwrap();
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Enabled, .. } if manifest.id == "one")));
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Disabled, .. } if manifest.id == "two")));
        assert!(entries.iter().any(|entry| matches!(entry, RegistryEntry::Installed { manifest, status: PluginStatus::Incompatible, .. } if manifest.id == "future")));
        assert!(
            entries
                .iter()
                .any(|entry| matches!(entry, RegistryEntry::Invalid { .. }))
        );
    }
}
