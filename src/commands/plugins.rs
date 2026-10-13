use std::error::Error;
use std::fmt;
use std::io;

use crate::config;
use crate::plugin::Permission;
use crate::plugin::registry::{self, PluginStatus, RegistryEntry};
use crate::render;

#[derive(Debug)]
pub enum PluginsCommandError {
    Usage,
    Configuration(String),
    Registry(registry::RegistryError),
    Output(io::Error),
}

pub fn plugins(args: &[String]) -> Result<(), PluginsCommandError> {
    if args != ["list"] {
        return Err(PluginsCommandError::Usage);
    }
    let config = config::load_user_config()
        .map_err(|error| PluginsCommandError::Configuration(error.to_string()))?;
    let entries = registry::discover_default(&config.plugins.enabled)
        .map_err(PluginsCommandError::Registry)?;
    render::stdout(format_entries(&entries)).map_err(PluginsCommandError::Output)
}

fn format_entries(entries: &[RegistryEntry]) -> String {
    if entries.is_empty() {
        return "No plugins installed.\n".into();
    }
    let mut output = format!("Installed plugins ({}):\n", entries.len());
    for entry in entries {
        match entry {
            RegistryEntry::Installed {
                compatibility,
                manifest,
                path,
                status,
            } => {
                let status = match status {
                    PluginStatus::Enabled => "enabled",
                    PluginStatus::Disabled => "disabled",
                    PluginStatus::Incompatible => "incompatible",
                };
                output.push_str(&format!(
                    "  {} {} [{}] - {} ({})\n",
                    manifest.id,
                    manifest.version,
                    status,
                    manifest.description,
                    path.display()
                ));
                output.push_str(&format!(
                    "    requested permissions: {}\n",
                    format_permissions(&manifest.permissions)
                ));
                if !compatibility.is_compatible() {
                    output.push_str(&format!("    compatibility: {compatibility}\n"));
                }
            }
            RegistryEntry::Invalid { path, message } => {
                output.push_str(&format!(
                    "  invalid [error] - {} ({message})\n",
                    path.display()
                ));
            }
        }
    }
    output
}

fn format_permissions(permissions: &[Permission]) -> String {
    if permissions.is_empty() {
        return "none".into();
    }
    let mut names = permissions
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    names.sort();
    names.join(", ")
}

impl fmt::Display for PluginsCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(formatter, "plugins: expected 'list'"),
            Self::Configuration(message) => write!(formatter, "plugins: {message}"),
            Self::Registry(error) => write!(formatter, "plugins: {error}"),
            Self::Output(error) => write!(formatter, "plugins: cannot write output: {error}"),
        }
    }
}

impl Error for PluginsCommandError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginManifest;
    use semver::{Version, VersionReq};

    #[test]
    fn formats_an_empty_registry() {
        assert_eq!(format_entries(&[]), "No plugins installed.\n");
    }

    #[test]
    fn rejects_unknown_actions() {
        assert_eq!(
            plugins(&[]).unwrap_err().to_string(),
            "plugins: expected 'list'"
        );
    }

    #[test]
    fn formats_requested_permissions_for_review() {
        assert_eq!(
            format_permissions(&[Permission::ProcessSpawn, Permission::FilesystemRead]),
            "filesystem.read, process.spawn"
        );
        assert_eq!(format_permissions(&[]), "none");
    }

    #[test]
    fn plugin_list_exposes_permissions_before_activation() {
        let entry = RegistryEntry::Installed {
            compatibility: crate::plugin::Compatibility::Compatible,
            manifest: Box::new(PluginManifest {
                schema_version: 1,
                id: "example.secure".into(),
                name: "Secure example".into(),
                version: Version::new(1, 0, 0),
                description: "Permission review fixture".into(),
                host_api: VersionReq::parse("^1").unwrap(),
                terminal_version: VersionReq::STAR,
                authors: Vec::new(),
                permissions: vec![Permission::NetworkConnect, Permission::ConfigRead],
                commands: Vec::new(),
                prompt_segments: Vec::new(),
            }),
            path: std::path::PathBuf::from("plugins/example"),
            status: PluginStatus::Disabled,
        };

        let output = format_entries(&[entry]);

        assert!(output.contains("example.secure 1.0.0 [disabled]"));
        assert!(output.contains("requested permissions: config.read, network.connect"));
    }

    #[test]
    fn plugin_list_explains_compatibility_failures() {
        let entry = RegistryEntry::Installed {
            compatibility: crate::plugin::Compatibility::TerminalMismatch {
                required: VersionReq::parse(">=9").unwrap(),
                current: Version::new(0, 3, 0),
            },
            manifest: Box::new(PluginManifest {
                schema_version: 1,
                id: "example.future".into(),
                name: "Future example".into(),
                version: Version::new(1, 0, 0),
                description: "Compatibility fixture".into(),
                host_api: VersionReq::parse("^1").unwrap(),
                terminal_version: VersionReq::parse(">=9").unwrap(),
                authors: Vec::new(),
                permissions: Vec::new(),
                commands: Vec::new(),
                prompt_segments: Vec::new(),
            }),
            path: std::path::PathBuf::from("plugins/future"),
            status: PluginStatus::Incompatible,
        };

        let output = format_entries(&[entry]);

        assert!(output.contains("example.future 1.0.0 [incompatible]"));
        assert!(output.contains("requires Terminal Rust >=9"));
        assert!(output.contains("0.3.0 is installed"));
        assert!(output.contains("update the plugin or Terminal Rust"));
    }
}
