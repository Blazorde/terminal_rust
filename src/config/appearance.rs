use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::shell_state::{
    load_persistent_shell_state, save_persistent_shell_state, shell_state_path,
};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appearance_sequences_use_vivid_defaults_and_custom_overrides() {
        let settings = AppearanceSettings {
            foreground: Some("#112233".into()),
            palette: BTreeMap::from([(1, "#AABBCC".into())]),
            ..Default::default()
        };
        let sequences = appearance_ansi_sequences_for_system(&settings, AppearanceTheme::Dark);

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
