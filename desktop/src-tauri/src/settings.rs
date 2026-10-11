use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

const SETTINGS_FILE: &str = "desktop-settings.json";

pub struct SettingsState {
    path: PathBuf,
    write_lock: Mutex<()>,
}

impl SettingsState {
    pub fn new(config_directory: PathBuf) -> Self {
        Self {
            path: config_directory.join(SETTINGS_FILE),
            write_lock: Mutex::new(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct DesktopSettings {
    pub version: u32,
    pub theme: DesktopTheme,
    pub font_family: FontFamily,
    pub font_size: u8,
    pub cursor_style: CursorStyle,
    pub cursor_blink: bool,
    pub window_density: WindowDensity,
    pub show_status_bar: bool,
}

impl Default for DesktopSettings {
    fn default() -> Self {
        Self {
            version: 1,
            theme: DesktopTheme::Graphite,
            font_family: FontFamily::System,
            font_size: 14,
            cursor_style: CursorStyle::Block,
            cursor_blink: true,
            window_density: WindowDensity::Comfortable,
            show_status_bar: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DesktopTheme {
    #[default]
    Graphite,
    Midnight,
    Paper,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FontFamily {
    #[default]
    System,
    Cascadia,
    Jetbrains,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowDensity {
    Compact,
    #[default]
    Comfortable,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLoad {
    pub settings: DesktopSettings,
    pub diagnostic: Option<String>,
    pub path: String,
}

pub fn load(state: &SettingsState) -> SettingsLoad {
    load_from_path(&state.path)
}

pub fn save(state: &SettingsState, settings: &DesktopSettings) -> Result<(), String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "desktop settings store is unavailable".to_string())?;
    save_to_path(&state.path, settings).map_err(|error| {
        format!(
            "cannot save desktop settings to {}: {error}",
            state.path.display()
        )
    })
}

fn load_from_path(path: &Path) -> SettingsLoad {
    let fallback = || SettingsLoad {
        settings: DesktopSettings::default(),
        diagnostic: None,
        path: path.display().to_string(),
    };

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return fallback(),
        Err(error) => {
            return SettingsLoad {
                diagnostic: Some(format!(
                    "Impossible de lire {} : {error}. Les réglages sûrs sont appliqués.",
                    path.display()
                )),
                ..fallback()
            };
        }
    };

    let settings = match serde_json::from_str::<DesktopSettings>(&content) {
        Ok(settings) => settings,
        Err(error) => {
            return SettingsLoad {
                diagnostic: Some(format!(
                    "Configuration invalide dans {} : {error}. Les réglages sûrs sont appliqués.",
                    path.display()
                )),
                ..fallback()
            };
        }
    };

    if let Err(message) = validate(&settings) {
        return SettingsLoad {
            diagnostic: Some(format!(
                "Configuration invalide dans {} : {message}. Les réglages sûrs sont appliqués.",
                path.display()
            )),
            ..fallback()
        };
    }

    SettingsLoad {
        settings,
        diagnostic: None,
        path: path.display().to_string(),
    }
}

fn save_to_path(path: &Path, settings: &DesktopSettings) -> io::Result<()> {
    validate(settings).map_err(|message| io::Error::new(io::ErrorKind::InvalidInput, message))?;
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "desktop settings path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)?;

    let serialized = serde_json::to_vec_pretty(settings)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let temporary = path.with_extension("json.tmp");
    let backup = path.with_extension("json.bak");

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)?;
    file.write_all(&serialized)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    drop(file);

    if path.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(path, &backup)?;
    }

    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() {
            let _ = fs::rename(&backup, path);
        }
        return Err(error);
    }

    if backup.exists() {
        // The new settings are already durable at this point. A stale backup is
        // harmless and must not turn a successful save into an error response.
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn validate(settings: &DesktopSettings) -> Result<(), String> {
    if settings.version != 1 {
        return Err(format!(
            "version {} non prise en charge (version attendue : 1)",
            settings.version
        ));
    }
    if !(12..=22).contains(&settings.font_size) {
        return Err("fontSize doit être compris entre 12 et 22".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "terminal-rust-desktop-{}-{nonce}",
                std::process::id()
            ))
            .join(name)
    }

    #[test]
    fn missing_settings_use_safe_defaults_without_a_diagnostic() {
        let path = fixture("missing.json");
        let loaded = load_from_path(&path);

        assert_eq!(loaded.settings, DesktopSettings::default());
        assert!(loaded.diagnostic.is_none());
    }

    #[test]
    fn invalid_settings_report_the_file_and_fall_back() {
        let path = fixture("invalid.json");
        fs::create_dir_all(path.parent().expect("fixture should have a parent"))
            .expect("fixture directory should be created");
        fs::write(&path, r#"{"version":1,"fontSize":99}"#).expect("fixture should be written");

        let loaded = load_from_path(&path);

        assert_eq!(loaded.settings, DesktopSettings::default());
        assert!(
            loaded
                .diagnostic
                .expect("invalid settings should report a diagnostic")
                .contains("invalid.json")
        );
    }

    #[test]
    fn valid_settings_round_trip_through_the_native_path() {
        let path = fixture("desktop-settings.json");
        let settings = DesktopSettings {
            theme: DesktopTheme::Paper,
            font_family: FontFamily::Jetbrains,
            font_size: 18,
            cursor_style: CursorStyle::Underline,
            cursor_blink: false,
            window_density: WindowDensity::Compact,
            show_status_bar: false,
            ..DesktopSettings::default()
        };

        save_to_path(&path, &settings).expect("settings should be saved");
        let loaded = load_from_path(&path);

        assert_eq!(loaded.settings, settings);
        assert!(loaded.diagnostic.is_none());
        assert!(!path.with_extension("json.tmp").exists());
        assert!(!path.with_extension("json.bak").exists());
    }
}
