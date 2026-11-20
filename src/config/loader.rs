use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::schema::AppConfig;
use super::validation;

const CONFIG_ENV: &str = "TERMINAL_RUST_CONFIG";

#[derive(Debug)]
pub enum ConfigError {
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

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "invalid TOML in {}: {source}", path.display())
            }
            Self::Validation { path, message } => {
                write!(
                    formatter,
                    "invalid configuration in {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl Error for ConfigError {}

pub fn load_user_config() -> Result<AppConfig, ConfigError> {
    let explicit = env::var_os(CONFIG_ENV).filter(|value| !value.is_empty());
    let path = explicit.map(PathBuf::from).or_else(default_config_path);
    let Some(path) = path else {
        return Ok(AppConfig::default());
    };

    if !path.exists() && env::var_os(CONFIG_ENV).is_none() {
        return Ok(AppConfig::default());
    }

    load_from_path(&path)
}

fn load_from_path(path: &Path) -> Result<AppConfig, ConfigError> {
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config = toml::from_str::<AppConfig>(&content).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    validation::validate(&config).map_err(|message| ConfigError::Validation {
        path: path.to_path_buf(),
        message,
    })?;
    Ok(config)
}

pub(super) fn default_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .map(|path| path.join("terminal-rust").join("config.toml"))
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
                    .join("config.toml")
            })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .map(|path| path.join(".config"))
            })
            .map(|path| path.join("terminal-rust").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(name: &str, content: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        let directory = env::temp_dir().join(format!(
            "terminal-rust-config-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&directory).expect("fixture directory should be created");
        let path = directory.join(name);
        fs::write(&path, content).expect("fixture should be written");
        path
    }

    #[test]
    fn loads_a_valid_partial_configuration_with_defaults() {
        let path = fixture("valid.toml", "version = 1\n[prompt]\nicons = \"never\"\n");
        let config = load_from_path(&path).expect("configuration should load");

        assert_eq!(
            config.prompt.icons,
            super::super::schema::IconSetting::Never
        );
        assert_eq!(config.theme, "default");
        assert_eq!(config.segments.symbol.value, ">");
    }

    #[test]
    fn reports_the_file_for_invalid_toml() {
        let path = fixture("invalid.toml", "version = [");
        let error = load_from_path(&path).expect_err("invalid TOML should fail");

        assert!(error.to_string().contains("invalid.toml"));
        assert!(error.to_string().contains("invalid TOML"));
    }

    #[test]
    fn rejects_an_unsupported_version() {
        let path = fixture("future.toml", "version = 2");
        let error = load_from_path(&path).expect_err("future version should fail");

        assert!(error.to_string().contains("expected 1"));
    }

    #[test]
    fn loads_enabled_plugins_and_preserves_their_settings() {
        let path = fixture(
            "plugins.toml",
            r#"
version = 1

[plugins]
enabled = ["git", "docker", "node"]

[plugins.grants]
git = ["command.register", "output.write"]

[plugins.git]
show_branch = true
show_dirty = true

[plugins.docker]
context = "desktop-linux"
"#,
        );
        let config = load_from_path(&path).expect("plugin configuration should load");

        assert!(config.plugins.enabled.iter().any(|plugin| plugin == "git"));
        assert!(config.plugins.enabled.iter().any(|plugin| plugin == "node"));
        assert!(!config.plugins.settings.contains_key("node"));
        assert_eq!(
            config.plugins.grants["git"],
            ["command.register", "output.write"]
        );
        assert_eq!(
            config
                .plugins
                .settings
                .get("git")
                .and_then(|settings| settings.get("show_dirty"))
                .and_then(toml::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            config
                .plugins
                .settings
                .get("docker")
                .and_then(|settings| settings.get("context"))
                .and_then(toml::Value::as_str),
            Some("desktop-linux")
        );
    }

    #[test]
    fn retains_settings_for_a_disabled_plugin() {
        let path = fixture(
            "disabled-plugin.toml",
            "version = 1\n[plugins.git]\nshow_branch = false\n",
        );
        let config = load_from_path(&path).expect("disabled plugin settings should load");

        assert!(!config.plugins.enabled.iter().any(|plugin| plugin == "git"));
        assert!(config.plugins.settings.contains_key("git"));
    }

    #[test]
    fn rejects_non_table_plugin_settings() {
        let path = fixture(
            "invalid-plugin-settings.toml",
            "version = 1\n[plugins]\nenabled = []\ngit = true\n",
        );
        let error = load_from_path(&path).expect_err("plugin settings must be a table");

        assert!(error.to_string().contains("invalid TOML"));
        assert!(error.to_string().contains("invalid-plugin-settings.toml"));
    }

    #[test]
    fn loads_structured_user_commands() {
        let path = fixture(
            "commands.toml",
            r#"
version = 1

[commands.git-summary]
kind = "builtin"
target = "git"
args = ["status", "--short", "--branch"]
append_args = false
description = "Show a concise Git summary"

[commands.serve]
kind = "process"
target = "npm"
args = ["run", "dev"]
"#,
        );
        let config = load_from_path(&path).expect("custom commands should load");

        assert_eq!(
            config.commands["git-summary"].kind,
            super::super::schema::UserCommandKind::Builtin
        );
        assert_eq!(config.commands["serve"].target, "npm");
        assert!(config.commands["serve"].append_args);
    }
}
