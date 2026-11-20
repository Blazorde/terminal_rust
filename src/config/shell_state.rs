use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::appearance::AppearanceSettings;
use super::loader::default_config_path;
use super::schema::UserCommandConfig;

const CONFIG_ENV: &str = "TERMINAL_RUST_CONFIG";
const SHELL_STATE_FILE: &str = "shell-state.toml";

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PersistentShellState {
    pub aliases: BTreeMap<String, UserCommandConfig>,
    pub startup_script: Option<String>,
    pub appearance: AppearanceSettings,
}

pub(crate) fn load_persistent_shell_state() -> Result<PersistentShellState, String> {
    let Some(path) = shell_state_path() else {
        return Ok(PersistentShellState::default());
    };
    match fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content)
            .map_err(|error| format!("invalid shell state in {}: {error}", path.display())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(PersistentShellState::default())
        }
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

pub(crate) fn save_aliases(aliases: BTreeMap<String, UserCommandConfig>) -> Result<(), String> {
    let mut state = load_persistent_shell_state()?;
    state.aliases = aliases;
    save_persistent_shell_state(&state)
}

pub(crate) fn save_startup_script(startup_script: Option<String>) -> Result<(), String> {
    let mut state = load_persistent_shell_state()?;
    state.startup_script = startup_script;
    save_persistent_shell_state(&state)
}

pub(super) fn save_persistent_shell_state(state: &PersistentShellState) -> Result<(), String> {
    let path = shell_state_path().ok_or_else(|| {
        "cannot determine the user configuration directory; set TERMINAL_RUST_CONFIG".to_string()
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let content = toml::to_string_pretty(state)
        .map_err(|error| format!("cannot serialize shell state: {error}"))?;
    let temporary = path.with_extension("toml.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| format!("cannot create {}: {error}", temporary.display()))?;
    file.write_all(content.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("cannot write {}: {error}", temporary.display()))?;
    drop(file);
    replace_file(&temporary, &path)
        .map_err(|error| format!("cannot save {}: {error}", path.display()))
}

fn replace_file(temporary: &Path, path: &Path) -> io::Result<()> {
    if !path.exists() {
        return fs::rename(temporary, path);
    }

    let backup = path.with_extension("toml.bak");
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    fs::rename(path, &backup)?;
    if let Err(error) = fs::rename(temporary, path) {
        let _ = fs::rename(&backup, path);
        return Err(error);
    }
    let _ = fs::remove_file(backup);
    Ok(())
}

pub(super) fn shell_state_path() -> Option<PathBuf> {
    if let Some(config_path) = env::var_os(CONFIG_ENV).filter(|path| !path.is_empty()) {
        let config_path = PathBuf::from(config_path);
        let directory = config_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        return Some(directory.join(SHELL_STATE_FILE));
    }
    default_config_path()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .map(|directory| directory.join(SHELL_STATE_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppearanceTheme;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn shell_state_round_trips_aliases_and_startup_script() {
        let path = std::env::temp_dir().join(format!(
            "terminal-rust-shell-state-{}-{}.toml",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should follow the epoch")
                .as_nanos()
        ));
        let state = PersistentShellState {
            aliases: BTreeMap::from([(
                "ll".into(),
                UserCommandConfig {
                    target: "ls".into(),
                    args: vec!["-la".into()],
                    ..Default::default()
                },
            )]),
            startup_script: Some("C:/scripts/start.trs".into()),
            appearance: AppearanceSettings {
                theme: AppearanceTheme::Light,
                foreground: Some("#112233".into()),
                background: Some("#F0E0D0".into()),
                palette: BTreeMap::from([(1, "#FF0000".into())]),
            },
        };
        let serialized = toml::to_string_pretty(&state).expect("state should serialize");
        fs::write(&path, serialized).expect("fixture should be written");

        let loaded = toml::from_str::<PersistentShellState>(&fs::read_to_string(&path).unwrap())
            .expect("state should deserialize");
        assert_eq!(loaded, state);
        fs::remove_file(path).expect("fixture should be removed");
    }

    #[test]
    fn old_shell_state_without_appearance_keeps_default_settings() {
        let state = toml::from_str::<PersistentShellState>(
            "aliases = {}\nstartup_script = \"C:/scripts/start.trs\"\n",
        )
        .expect("older shell-state files remain valid");

        assert_eq!(state.appearance, AppearanceSettings::default());
        assert_eq!(
            state.startup_script.as_deref(),
            Some("C:/scripts/start.trs")
        );
    }
}
