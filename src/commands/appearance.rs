use std::io;

use crate::config::{
    AppearanceSettings, AppearanceTheme, appearance_ansi_sequences, appearance_palette,
    load_appearance_settings, save_appearance_settings, validate_appearance_settings,
};
use crate::render;

const USAGE: &str = "appearance: expected show, theme <system|dark|light>, foreground <#RRGGBB>, background <#RRGGBB>, color <0-15> <#RRGGBB>, or reset";

pub fn appearance(args: &[String]) -> Result<(), String> {
    match args {
        [] => show(),
        [arg] if arg == "show" => show(),
        [action, theme] if action == "theme" => set_theme(theme),
        [action, color] if action == "foreground" => {
            update(|settings| settings.foreground = Some(color.clone()))
        }
        [action, color] if action == "background" => {
            update(|settings| settings.background = Some(color.clone()))
        }
        [action, index, color] if action == "color" => {
            let index = index
                .parse::<u8>()
                .map_err(|_| "appearance: ANSI color index must be between 0 and 15".to_string())?;
            if index > 15 {
                return Err("appearance: ANSI color index must be between 0 and 15".into());
            }
            update(|settings| {
                settings.palette.insert(index, color.clone());
            })
        }
        [action] if action == "reset" => {
            save_appearance_settings(AppearanceSettings::default())?;
            apply(AppearanceSettings::default())
        }
        _ => Err(USAGE.into()),
    }
}

fn show() -> Result<(), String> {
    let settings = load_appearance_settings()?;
    let mut output = format!("Theme: {}\n", theme_name(settings.theme));
    output.push_str(&format!(
        "Foreground: {}\nBackground: {}\nANSI palette:\n",
        settings.foreground.as_deref().unwrap_or("system default"),
        settings.background.as_deref().unwrap_or("system default")
    ));
    for (index, default_color) in appearance_palette(&settings).iter().enumerate() {
        let color = settings
            .palette
            .get(&(index as u8))
            .map_or(*default_color, String::as_str);
        output.push_str(&format!("  {index:>2}: {color}\n"));
    }
    render::stdout(output).map_err(output_error)
}

fn set_theme(value: &str) -> Result<(), String> {
    let theme = match value {
        "system" => AppearanceTheme::System,
        "dark" => AppearanceTheme::Dark,
        "light" => AppearanceTheme::Light,
        _ => return Err("appearance: theme must be system, dark, or light".into()),
    };
    update(|settings| settings.theme = theme)
}

fn update(change: impl FnOnce(&mut AppearanceSettings)) -> Result<(), String> {
    let mut settings = load_appearance_settings()?;
    change(&mut settings);
    validate_appearance_settings(&settings)?;
    save_appearance_settings(settings.clone())?;
    apply(settings)
}

fn apply(settings: AppearanceSettings) -> Result<(), String> {
    render::stdout(appearance_ansi_sequences(&settings))
        .and_then(|()| {
            render::stdout_line(format!(
                "Appearance saved. Theme: {}. Use 'appearance show' to inspect settings.",
                theme_name(settings.theme)
            ))
        })
        .map_err(output_error)
}

fn theme_name(theme: AppearanceTheme) -> &'static str {
    match theme {
        AppearanceTheme::System => "system",
        AppearanceTheme::Dark => "dark",
        AppearanceTheme::Light => "light",
    }
}

fn output_error(error: io::Error) -> String {
    format!("appearance: cannot write terminal output: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_theme_and_color_indices_are_rejected_before_saving() {
        assert!(
            set_theme("sepia")
                .expect_err("unknown themes should fail")
                .contains("system, dark, or light")
        );
        assert!(
            appearance(&["color".into(), "16".into(), "#FFFFFF".into()])
                .expect_err("out-of-range ANSI colors should fail")
                .contains("between 0 and 15")
        );
    }
}
