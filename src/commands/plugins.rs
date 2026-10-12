use std::error::Error;
use std::fmt;
use std::io;

use crate::config;
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
}
