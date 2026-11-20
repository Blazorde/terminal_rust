use std::collections::BTreeMap;

use portable_pty::CommandBuilder;
use terminal_rust::appearance::{self, AppearanceSettings, AppearanceTheme};

pub const USAGE: &str = "Usage: terminal-rust-native-prototype [--theme system|dark|light] [--foreground #RRGGBB] [--background #RRGGBB] [--color INDEX=#RRGGBB]";

#[derive(Default, Clone)]
pub struct LaunchOverrides {
    theme: Option<AppearanceTheme>,
    foreground: Option<String>,
    background: Option<String>,
    palette: BTreeMap<u8, String>,
}

impl LaunchOverrides {
    pub fn apply(&self, settings: &mut AppearanceSettings) {
        if let Some(theme) = self.theme {
            settings.theme = theme;
        }
        if let Some(foreground) = &self.foreground {
            settings.foreground = Some(foreground.clone());
        }
        if let Some(background) = &self.background {
            settings.background = Some(background.clone());
        }
        settings.palette.extend(self.palette.clone());
    }

    pub fn apply_to_command(&self, command: &mut CommandBuilder) {
        if let Some(theme) = self.theme {
            command.env("TERMINAL_RUST_APPEARANCE_THEME", theme_label(theme));
        }
        if let Some(foreground) = &self.foreground {
            command.env("TERMINAL_RUST_APPEARANCE_FOREGROUND", foreground);
        }
        if let Some(background) = &self.background {
            command.env("TERMINAL_RUST_APPEARANCE_BACKGROUND", background);
        }
        for (index, color) in &self.palette {
            command.env(format!("TERMINAL_RUST_APPEARANCE_COLOR_{index}"), color);
        }
    }
}

pub fn parse_options(arguments: &[String]) -> Result<LaunchOverrides, String> {
    let mut options = LaunchOverrides::default();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--theme" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                options.theme = Some(parse_theme(value)?);
            }
            "--foreground" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                appearance::validate_hex_color(value)?;
                options.foreground = Some(value.clone());
            }
            "--background" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                appearance::validate_hex_color(value)?;
                options.background = Some(value.clone());
            }
            "--color" => {
                let value = arguments.next().ok_or_else(|| USAGE.to_string())?;
                let (index, color) = value
                    .split_once('=')
                    .ok_or_else(|| "expected --color INDEX=#RRGGBB".to_string())?;
                let index = index
                    .parse::<u8>()
                    .map_err(|_| "ANSI color index must be between 0 and 15".to_string())?;
                if index > 15 {
                    return Err("ANSI color index must be between 0 and 15".into());
                }
                appearance::validate_hex_color(color)?;
                options.palette.insert(index, color.to_string());
            }
            _ => return Err(format!("unknown option '{argument}'\n{USAGE}")),
        }
    }
    Ok(options)
}

fn parse_theme(value: &str) -> Result<AppearanceTheme, String> {
    match value {
        "system" => Ok(AppearanceTheme::System),
        "dark" => Ok(AppearanceTheme::Dark),
        "light" => Ok(AppearanceTheme::Light),
        _ => Err(format!(
            "invalid theme '{value}'; expected system, dark, or light"
        )),
    }
}

pub fn theme_preference(theme: AppearanceTheme) -> eframe::egui::ThemePreference {
    match theme {
        AppearanceTheme::System => eframe::egui::ThemePreference::System,
        AppearanceTheme::Dark => eframe::egui::ThemePreference::Dark,
        AppearanceTheme::Light => eframe::egui::ThemePreference::Light,
    }
}

pub fn theme_label(theme: AppearanceTheme) -> &'static str {
    match theme {
        AppearanceTheme::System => "system",
        AppearanceTheme::Dark => "dark",
        AppearanceTheme::Light => "light",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_options_validate_and_collect_custom_palette_entries() {
        let options = parse_options(&[
            "--theme".into(),
            "light".into(),
            "--color".into(),
            "1=#FF1020".into(),
        ])
        .expect("valid launch options should parse");
        assert_eq!(options.theme, Some(AppearanceTheme::Light));
        assert_eq!(options.palette.get(&1).map(String::as_str), Some("#FF1020"));
    }

    #[test]
    fn launch_options_reject_invalid_palette_values() {
        assert!(parse_options(&["--color".into(), "16=#FFFFFF".into()]).is_err());
        assert!(parse_options(&["--foreground".into(), "red".into()]).is_err());
    }

    #[test]
    fn launch_overrides_do_not_mutate_saved_appearance_settings() {
        let options = LaunchOverrides {
            theme: Some(AppearanceTheme::Dark),
            ..Default::default()
        };
        let saved = AppearanceSettings::default();
        let mut effective = saved.clone();
        options.apply(&mut effective);
        assert_eq!(saved.theme, AppearanceTheme::System);
        assert_eq!(effective.theme, AppearanceTheme::Dark);
    }
}
