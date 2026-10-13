use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::Deserialize;

use super::PermissionSet;
use super::command::CommandContribution;
use super::prompt_segment::PromptSegmentContribution;

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
    #[serde(default = "any_version")]
    pub terminal_version: VersionReq,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub prompt_segments: Vec<PromptSegmentContribution>,
}

fn any_version() -> VersionReq {
    VersionReq::STAR
}

fn matches_terminal_version(requirement: &VersionReq, current: &Version) -> bool {
    if requirement.matches(current) {
        return true;
    }

    if current.pre.is_empty() {
        return false;
    }

    let release = Version::new(current.major, current.minor, current.patch);
    requirement.matches(&release)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Compatibility {
    Compatible,
    HostApiMismatch {
        required: VersionReq,
        current: Version,
    },
    TerminalMismatch {
        required: VersionReq,
        current: Version,
    },
}

impl Compatibility {
    pub(crate) fn is_compatible(&self) -> bool {
        matches!(self, Self::Compatible)
    }
}

impl fmt::Display for Compatibility {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compatible => formatter.write_str("compatible"),
            Self::HostApiMismatch { required, current } => write!(
                formatter,
                "requires plugin API {required}, but the host provides {current}; update the plugin or Terminal Rust"
            ),
            Self::TerminalMismatch { required, current } => write!(
                formatter,
                "requires Terminal Rust {required}, but {current} is installed; update the plugin or Terminal Rust"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
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
    #[serde(rename = "filesystem.read")]
    FilesystemRead,
    #[serde(rename = "filesystem.write")]
    FilesystemWrite,
    #[serde(rename = "process.spawn")]
    ProcessSpawn,
    #[serde(rename = "network.connect")]
    NetworkConnect,
}

impl Permission {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::ContextCwdRead => "context.cwd.read",
            Self::ContextEnvRead => "context.env.read",
            Self::ConfigRead => "config.read",
            Self::CommandRegister => "command.register",
            Self::CompletionRegister => "completion.register",
            Self::OutputWrite => "output.write",
            Self::PromptSegmentRegister => "prompt.segment.register",
            Self::FilesystemRead => "filesystem.read",
            Self::FilesystemWrite => "filesystem.write",
            Self::ProcessSpawn => "process.spawn",
            Self::NetworkConnect => "network.connect",
        }
    }

    pub(crate) fn from_name(value: &str) -> Option<Self> {
        Some(match value {
            "context.cwd.read" => Self::ContextCwdRead,
            "context.env.read" => Self::ContextEnvRead,
            "config.read" => Self::ConfigRead,
            "command.register" => Self::CommandRegister,
            "completion.register" => Self::CompletionRegister,
            "output.write" => Self::OutputWrite,
            "prompt.segment.register" => Self::PromptSegmentRegister,
            "filesystem.read" => Self::FilesystemRead,
            "filesystem.write" => Self::FilesystemWrite,
            "process.spawn" => Self::ProcessSpawn,
            "network.connect" => Self::NetworkConnect,
            _ => return None,
        })
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
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

    pub(crate) fn compatibility(&self) -> Compatibility {
        let host_api = Version::new(1, 0, 0);
        if !self.host_api.matches(&host_api) {
            return Compatibility::HostApiMismatch {
                required: self.host_api.clone(),
                current: host_api,
            };
        }
        let terminal = Version::parse(env!("CARGO_PKG_VERSION"))
            .expect("Cargo package versions are valid semantic versions");
        if !matches_terminal_version(&self.terminal_version, &terminal) {
            return Compatibility::TerminalMismatch {
                required: self.terminal_version.clone(),
                current: terminal,
            };
        }
        Compatibility::Compatible
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
        let mut permissions = std::collections::HashSet::new();
        for permission in &self.permissions {
            if !permissions.insert(permission) {
                return Err(invalid(format!("duplicate permission '{permission}'")));
            }
        }
        for command in &self.commands {
            command
                .validate()
                .map_err(|message| invalid(format!("command '{}': {message}", command.name)))?;
        }
        let declared_permissions = PermissionSet::for_manifest(self, &self.permissions);
        if !self.commands.is_empty()
            && declared_permissions
                .authorize(Permission::CommandRegister)
                .is_err()
        {
            return Err(invalid(
                "commands require the 'command.register' permission".into(),
            ));
        }
        if self
            .commands
            .iter()
            .any(|command| command.response.is_some())
            && declared_permissions
                .authorize(Permission::OutputWrite)
                .is_err()
        {
            return Err(invalid(
                "declarative command responses require the 'output.write' permission".into(),
            ));
        }
        let mut segment_names = std::collections::HashSet::new();
        for segment in &self.prompt_segments {
            segment.validate().map_err(|message| {
                invalid(format!("prompt segment '{}': {message}", segment.name))
            })?;
            if !segment_names.insert(&segment.name) {
                return Err(invalid(format!(
                    "duplicate prompt segment '{}'",
                    segment.name
                )));
            }
        }
        if !self.prompt_segments.is_empty()
            && declared_permissions
                .authorize(Permission::PromptSegmentRegister)
                .is_err()
        {
            return Err(invalid(
                "prompt_segments require the 'prompt.segment.register' permission".into(),
            ));
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
terminal_version = ">=0.3, <2.0"
permissions = ["command.register", "output.write", "prompt.segment.register"]
[[commands]]
name = "git-clean-check"
summary = "Check whether the repository is clean"

[[prompt_segments]]
name = "git-review"
summary = "Pull request review state"
timeout_ms = 20
cache_ttl_ms = 5000
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
terminal_version = ">=0.3, <2.0"
permissions = ["terminal.control"]
"#,
        );
        let error = PluginManifest::load(&path).unwrap_err().to_string();
        assert!(error.contains(path.to_string_lossy().as_ref()));
        assert!(error.contains("unknown variant"));
    }

    #[test]
    fn accepts_sensitive_permissions_as_requests_not_implicit_grants() {
        let path = fixture(
            r#"schema_version = 1
id = "scoped-tools"
name = "Scoped tools"
version = "1.0.0"
description = "Requests host services"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
permissions = ["filesystem.read", "filesystem.write", "process.spawn", "network.connect"]
"#,
        );

        let manifest = PluginManifest::load(&path).unwrap();

        assert!(manifest.permissions.contains(&Permission::FilesystemRead));
        assert!(manifest.permissions.contains(&Permission::NetworkConnect));
    }

    #[test]
    fn rejects_duplicate_permissions() {
        let path = fixture(
            r#"schema_version = 1
id = "duplicate"
name = "Duplicate"
version = "1.0.0"
description = "Duplicate permissions"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
permissions = ["config.read", "config.read"]
"#,
        );

        let error = PluginManifest::load(&path).unwrap_err().to_string();

        assert!(error.contains("duplicate permission 'config.read'"));
    }

    #[test]
    fn blocks_a_contribution_without_its_declared_permission() {
        let path = fixture(
            r#"schema_version = 1
id = "missing-permission"
name = "Missing permission"
version = "1.0.0"
description = "Invalid command contribution"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"

[[commands]]
name = "hello"
summary = "Say hello"
"#,
        );

        let error = PluginManifest::load(&path).unwrap_err().to_string();

        assert!(error.contains("commands require the 'command.register' permission"));
    }

    #[test]
    fn declarative_response_requires_output_permission() {
        let path = fixture(
            r#"schema_version = 1
id = "missing-output"
name = "Missing output"
version = "1.0.0"
description = "Invalid declarative command"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
permissions = ["command.register"]

[[commands]]
name = "hello"
summary = "Say hello"
response = "hello"
"#,
        );

        let error = PluginManifest::load(&path).unwrap_err().to_string();

        assert!(error.contains("responses require the 'output.write' permission"));
    }

    #[test]
    fn rejects_terminal_control_in_a_declarative_response() {
        let path = fixture(
            "schema_version = 1\nid = \"control\"\nname = \"Control\"\nversion = \"1.0.0\"\ndescription = \"Invalid response\"\nhost_api = \"^1.0\"\npermissions = [\"command.register\", \"output.write\"]\n[[commands]]\nname = \"hello\"\nsummary = \"Say hello\"\nresponse = \"\\u001b[31munsafe\"\n",
        );

        let error = PluginManifest::load(&path).unwrap_err().to_string();

        assert!(error.contains("without NUL or escape characters"));
    }

    #[test]
    fn reports_host_api_and_terminal_version_mismatches() {
        let mut manifest = PluginManifest {
            schema_version: 1,
            id: "compatibility".into(),
            name: "Compatibility".into(),
            version: Version::new(1, 0, 0),
            description: "Compatibility fixture".into(),
            host_api: VersionReq::parse("^2").unwrap(),
            terminal_version: VersionReq::STAR,
            authors: Vec::new(),
            permissions: Vec::new(),
            commands: Vec::new(),
            prompt_segments: Vec::new(),
        };

        let api_error = manifest.compatibility().to_string();
        assert!(api_error.contains("requires plugin API ^2"));
        assert!(api_error.contains("host provides 1.0.0"));

        manifest.host_api = VersionReq::parse("^1").unwrap();
        manifest.terminal_version = VersionReq::parse(">=9").unwrap();
        let terminal_error = manifest.compatibility().to_string();
        assert!(terminal_error.contains("requires Terminal Rust >=9"));
        assert!(terminal_error.contains(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn legacy_schema_one_manifest_defaults_to_any_terminal_version() {
        let path = fixture(
            r#"schema_version = 1
id = "legacy"
name = "Legacy"
version = "1.0.0"
description = "Legacy schema one manifest"
host_api = "^1.0"
"#,
        );

        let manifest = PluginManifest::load(&path).unwrap();

        assert_eq!(manifest.terminal_version, VersionReq::STAR);
        assert_eq!(manifest.compatibility(), Compatibility::Compatible);
    }

    #[test]
    fn stable_terminal_ranges_accept_a_prerelease_of_the_same_version() {
        let requirement = VersionReq::parse(">=0.3, <2.0").unwrap();
        let alpha = Version::parse("1.0.0-alpha.1").unwrap();

        assert!(matches_terminal_version(&requirement, &alpha));
    }

    #[test]
    fn repository_example_manifest_stays_valid() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("plugins")
            .join("example-plugin")
            .join(MANIFEST_FILE);

        let manifest = PluginManifest::load(&path).unwrap();
        assert_eq!(manifest.id, "example.hello");
        assert_eq!(manifest.commands[0].name, "hello");
    }
}
