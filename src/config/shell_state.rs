use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::loader::default_config_path;
use super::schema::UserCommandConfig;

const CONFIG_ENV: &str = "TERMINAL_RUST_CONFIG";
const SHELL_STATE_FILE: &str = "shell-state.toml";

pub const VIVID_ANSI_PALETTE: [&str; 16] = [
    "#282A36", "#FF5555", "#50FA7B", "#F1FA8C", "#BD93F9", "#FF79C6", "#8BE9FD", "#F8F8F2",
    "#6272A4", "#FF6E6E", "#69FF94", "#FFFFA5", "#D6ACFF", "#FF92DF", "#A4FFFF", "#FFFFFF",
];
pub const VIVID_LIGHT_ANSI_PALETTE: [&str; 16] = [
    "#282A36", "#C2185B", "#1B7F3A", "#7A5C00", "#1D55A5", "#9A248F", "#006B70", "#6B7280",
    "#596275", "#D61F45", "#167D34", "#856404", "#0F55AA", "#B0178B", "#006E7D", "#282A36",
];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceSettings {
    pub theme: AppearanceTheme,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub palette: BTreeMap<u8, String>,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: AppearanceTheme::System,
            foreground: None,
            background: None,
            palette: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AppearanceTheme {
    #[default]
    System,
    Dark,
    Light,
}

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

pub fn load_appearance_settings() -> Result<AppearanceSettings, String> {
    let settings = load_persistent_shell_state()?.appearance;
    validate_appearance_settings(&settings)?;
    Ok(settings)
}

pub fn save_appearance_settings(settings: AppearanceSettings) -> Result<(), String> {
    validate_appearance_settings(&settings)?;
    let mut state = load_persistent_shell_state()?;
    state.appearance = settings;
    save_persistent_shell_state(&state)
}

pub fn appearance_settings_path() -> Option<PathBuf> {
    shell_state_path()
}

pub fn validate_appearance_settings(settings: &AppearanceSettings) -> Result<(), String> {
    for (name, color) in [
        ("foreground", settings.foreground.as_deref()),
        ("background", settings.background.as_deref()),
    ] {
        if let Some(color) = color {
            validate_hex_color(color).map_err(|error| format!("{name}: {error}"))?;
        }
    }
    for (index, color) in &settings.palette {
        if *index > 15 {
            return Err(format!(
                "ANSI color index {index} is outside the supported range 0-15"
            ));
        }
        validate_hex_color(color).map_err(|error| format!("ANSI color {index}: {error}"))?;
    }
    Ok(())
}

pub fn apply_appearance_environment(settings: &mut AppearanceSettings) -> Result<(), String> {
    if let Some(theme) = env::var_os("TERMINAL_RUST_APPEARANCE_THEME") {
        settings.theme = match theme.to_string_lossy().as_ref() {
            "system" => AppearanceTheme::System,
            "dark" => AppearanceTheme::Dark,
            "light" => AppearanceTheme::Light,
            value => return Err(format!("invalid appearance theme '{value}'")),
        };
    }
    if let Some(foreground) = env::var_os("TERMINAL_RUST_APPEARANCE_FOREGROUND") {
        settings.foreground = Some(foreground.to_string_lossy().into_owned());
    }
    if let Some(background) = env::var_os("TERMINAL_RUST_APPEARANCE_BACKGROUND") {
        settings.background = Some(background.to_string_lossy().into_owned());
    }
    for index in 0..16 {
        let variable = format!("TERMINAL_RUST_APPEARANCE_COLOR_{index}");
        if let Some(color) = env::var_os(variable) {
            settings
                .palette
                .insert(index, color.to_string_lossy().into_owned());
        }
    }
    validate_appearance_settings(settings)
}

pub fn validate_hex_color(color: &str) -> Result<(), String> {
    let digits = color.strip_prefix('#').unwrap_or(color);
    if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "expected a six-digit RGB color such as #89B4FA, got '{color}'"
        ));
    }
    Ok(())
}

pub fn appearance_ansi_sequences(settings: &AppearanceSettings) -> String {
    let system_theme =
        if env::var("TERMINAL_RUST_APPEARANCE_SYSTEM_RESOLVED").as_deref() == Ok("light") {
            AppearanceTheme::Light
        } else {
            AppearanceTheme::Dark
        };
    appearance_sequences(
        settings,
        system_theme,
        env::var_os("TERMINAL_RUST_NATIVE_UI").is_some(),
    )
}

pub fn appearance_ansi_sequences_for_system(
    settings: &AppearanceSettings,
    system_theme: AppearanceTheme,
) -> String {
    appearance_sequences(settings, system_theme, true)
}

fn appearance_sequences(
    settings: &AppearanceSettings,
    system_theme: AppearanceTheme,
    native_ui: bool,
) -> String {
    let mut output = String::new();
    let effective_theme = match settings.theme {
        AppearanceTheme::System => system_theme,
        theme => theme,
    };
    let (default_foreground, default_background, palette) = match effective_theme {
        AppearanceTheme::Light => ("#1D1A16", "#FFFDF9", &VIVID_LIGHT_ANSI_PALETTE),
        AppearanceTheme::Dark | AppearanceTheme::System => {
            ("#F8F8F2", "#282A36", &VIVID_ANSI_PALETTE)
        }
    };
    if native_ui {
        output.push_str(&osc_color(
            10,
            settings.foreground.as_deref().unwrap_or(default_foreground),
        ));
        output.push_str(&osc_color(
            11,
            settings.background.as_deref().unwrap_or(default_background),
        ));
    } else {
        output.push_str(&settings.foreground.as_deref().map_or_else(
            || "\u{1b}]110\u{7}".to_string(),
            |color| osc_color(10, color),
        ));
        output.push_str(&settings.background.as_deref().map_or_else(
            || "\u{1b}]111\u{7}".to_string(),
            |color| osc_color(11, color),
        ));
    }
    for (index, default_color) in palette.iter().enumerate() {
        let color = settings
            .palette
            .get(&(index as u8))
            .map_or(*default_color, String::as_str);
        output.push_str(&format!(
            "\u{1b}]4;{index};rgb:{}\u{7}",
            rgb_components(color)
        ));
    }
    output
}

pub fn appearance_palette(settings: &AppearanceSettings) -> &'static [&'static str; 16] {
    match effective_theme(settings.theme) {
        AppearanceTheme::Light => &VIVID_LIGHT_ANSI_PALETTE,
        AppearanceTheme::Dark | AppearanceTheme::System => &VIVID_ANSI_PALETTE,
    }
}

fn effective_theme(theme: AppearanceTheme) -> AppearanceTheme {
    match theme {
        AppearanceTheme::System
            if env::var("TERMINAL_RUST_APPEARANCE_SYSTEM_RESOLVED").as_deref() == Ok("light") =>
        {
            AppearanceTheme::Light
        }
        AppearanceTheme::System => AppearanceTheme::Dark,
        theme => theme,
    }
}

fn osc_color(operation: u8, color: &str) -> String {
    format!("\u{1b}]{operation};rgb:{}\u{7}", rgb_components(color))
}

fn rgb_components(color: &str) -> String {
    let digits = color.strip_prefix('#').unwrap_or(color);
    format!("{}/{}/{}", &digits[0..2], &digits[2..4], &digits[4..6])
}

fn save_persistent_shell_state(state: &PersistentShellState) -> Result<(), String> {
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

fn shell_state_path() -> Option<PathBuf> {
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

    #[test]
    fn appearance_sequences_use_vivid_defaults_and_custom_overrides() {
        let settings = AppearanceSettings {
            foreground: Some("#112233".into()),
            palette: BTreeMap::from([(1, "#AABBCC".into())]),
            ..Default::default()
        };
        let sequences = appearance_ansi_sequences(&settings);

        assert!(sequences.starts_with("\u{1b}]10;rgb:11/22/33\u{7}"));
        assert!(sequences.contains("\u{1b}]4;1;rgb:AA/BB/CC\u{7}"));
        assert!(sequences.contains("\u{1b}]4;2;rgb:50/FA/7B\u{7}"));
    }

    #[test]
    fn system_theme_uses_a_contrast_adjusted_light_palette_when_needed() {
        let settings = AppearanceSettings::default();
        let sequences = appearance_ansi_sequences_for_system(&settings, AppearanceTheme::Light);

        assert!(sequences.contains("\u{1b}]10;rgb:1D/1A/16\u{7}"));
        assert!(sequences.contains("\u{1b}]11;rgb:FF/FD/F9\u{7}"));
        assert!(sequences.contains("\u{1b}]4;1;rgb:C2/18/5B\u{7}"));
    }

    #[test]
    fn appearance_colors_must_be_rgb_hex_and_palette_indices_are_bounded() {
        assert!(validate_hex_color("#89B4FA").is_ok());
        assert!(validate_hex_color("89B4FA").is_ok());
        assert!(validate_hex_color("#89B4F").is_err());
        assert!(validate_hex_color("#GG0000").is_err());
        assert!(
            validate_appearance_settings(&AppearanceSettings {
                palette: BTreeMap::from([(16, "#FFFFFF".into())]),
                ..Default::default()
            })
            .is_err()
        );
    }
}
