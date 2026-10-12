use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::Deserialize;

pub(crate) const MANIFEST_FILE: &str = "terminal-rust-plugin.toml";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PluginManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: Version,
    pub description: String,
    pub host_api: VersionReq,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub(crate) enum Permission {
    #[serde(rename = "context.cwd.read")]
    ContextCwdRead,
    #[serde(rename = "context.env.read")]
    ContextEnvRead,
    #[serde(rename = "config.read")]
    ConfigRead,
    #[serde(rename = "command.register")]
    CommandRegister,
    #[serde(rename = "completion.register")]
    CompletionRegister,
    #[serde(rename = "output.write")]
    OutputWrite,
    #[serde(rename = "prompt.segment.register")]
    PromptSegmentRegister,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandContribution {
    pub name: String,
    pub summary: String,
}

#[derive(Debug)]
pub(crate) enum ManifestError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    Validation {
        path: PathBuf,
        message: String,
    },
}

impl PluginManifest {
    pub(crate) fn load(path: &Path) -> Result<Self, ManifestError> {
        let content = fs::read_to_string(path).map_err(|source| ManifestError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let manifest = toml::from_str::<Self>(&content).map_err(|source| ManifestError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        manifest.validate(path)?;
        Ok(manifest)
    }

    fn validate(&self, path: &Path) -> Result<(), ManifestError> {
        let invalid = |message: String| ManifestError::Validation {
            path: path.to_path_buf(),
            message,
        };
        if self.schema_version != 1 {
            return Err(invalid(format!(
                "unsupported schema version {} (expected 1)",
                self.schema_version
            )));
        }
        if !is_valid_id(&self.id) {
            return Err(invalid(format!("invalid plugin id '{}'", self.id)));
        }
        if self.name.trim().is_empty() || self.description.trim().is_empty() {
            return Err(invalid("name and description must not be empty".into()));
        }
        for command in &self.commands {
            if !is_valid_command_name(&command.name) || command.summary.trim().is_empty() {
                return Err(invalid(format!(
                    "command '{}' must have a lowercase name and a non-empty summary",
                    command.name
                )));
            }
        }
        Ok(())
    }
}

pub(crate) fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn is_valid_command_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'_'
        })
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "invalid TOML in {}: {source}", path.display())
            }
            Self::Validation { path, message } => {
                write!(formatter, "invalid manifest {}: {message}", path.display())
            }
        }
    }
}

impl Error for ManifestError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(content: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("terminal-rust-manifest-{nonce}.toml"));
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn parses_a_valid_manifest() {
        let path = fixture(
            r#"schema_version = 1
id = "acme.git-tools"
name = "Git tools"
version = "0.1.0"
description = "Safe Git helpers"
host_api = "^1.0"
permissions = ["command.register", "output.write"]
[[commands]]
name = "git-clean-check"
summary = "Check whether the repository is clean"
"#,
        );
        let manifest = PluginManifest::load(&path).unwrap();
        assert_eq!(manifest.id, "acme.git-tools");
        assert_eq!(manifest.version, Version::new(0, 1, 0));
    }

    #[test]
    fn reports_unknown_permissions_with_the_file() {
        let path = fixture(
            r#"schema_version = 1
id = "unsafe"
name = "Unsafe"
version = "1.0.0"
description = "Invalid permission"
host_api = "^1.0"
permissions = ["process.spawn"]
"#,
        );
        let error = PluginManifest::load(&path).unwrap_err().to_string();
        assert!(error.contains(path.to_string_lossy().as_ref()));
        assert!(error.contains("unknown variant"));
    }
}
