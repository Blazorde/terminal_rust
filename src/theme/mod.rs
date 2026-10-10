use std::error::Error;
use std::fmt;

use serde::Deserialize;

use crate::config::IconSetting;
use crate::render::ansi::AnsiColor;
use crate::render::palette::Palette;
use crate::render::separators::Separator;

const DEFAULT_THEME: &str = include_str!("../../config/themes/default.toml");
const MINIMAL_THEME: &str = include_str!("../../config/themes/minimal.toml");
const GIT_FOCUS_THEME: &str = include_str!("../../config/themes/git-focus.toml");
const POWERLINE_THEME: &str = include_str!("../../config/themes/powerline.toml");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub palette: Palette,
    pub icons: IconSetting,
    pub separator: Option<Separator>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ThemeError {
    Unknown(String),
    Parse { name: String, message: String },
    Invalid { name: String, message: String },
}

impl fmt::Display for ThemeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(
                formatter,
                "unknown theme '{name}' (available: default, minimal, git-focus, powerline)"
            ),
            Self::Parse { name, message } => {
                write!(formatter, "cannot parse built-in theme '{name}': {message}")
            }
            Self::Invalid { name, message } => {
                write!(formatter, "invalid built-in theme '{name}': {message}")
            }
        }
    }
}

impl Error for ThemeError {}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    version: u32,
    name: String,
    #[serde(default)]
    icons: IconSetting,
    palette: ThemePalette,
    separator: Option<ThemeSeparator>,
}

#[derive(Debug, Deserialize)]
struct ThemePalette {
    primary: String,
    secondary: String,
    success: String,
    warning: String,
    error: String,
    muted: String,
}

#[derive(Debug, Deserialize)]
struct ThemeSeparator {
    glyph: String,
    fallback: String,
}

pub fn load_builtin(name: &str) -> Result<Theme, ThemeError> {
    let source = match name {
        "default" => DEFAULT_THEME,
        "minimal" => MINIMAL_THEME,
        "git-focus" => GIT_FOCUS_THEME,
        "powerline" => POWERLINE_THEME,
        _ => return Err(ThemeError::Unknown(name.to_string())),
    };
    let definition = toml::from_str::<ThemeFile>(source).map_err(|error| ThemeError::Parse {
        name: name.to_string(),
        message: error.to_string(),
    })?;
    validate(name, &definition)?;

    Ok(Theme {
        palette: Palette::new(
            color(name, "primary", &definition.palette.primary)?,
            color(name, "secondary", &definition.palette.secondary)?,
            color(name, "success", &definition.palette.success)?,
            color(name, "warning", &definition.palette.warning)?,
            color(name, "error", &definition.palette.error)?,
            color(name, "muted", &definition.palette.muted)?,
        ),
        icons: definition.icons,
        separator: definition
            .separator
            .map(|separator| Separator::new(separator.glyph, separator.fallback)),
    })
}

fn validate(requested_name: &str, definition: &ThemeFile) -> Result<(), ThemeError> {
    if definition.version != 1 {
        return Err(ThemeError::Invalid {
            name: requested_name.to_string(),
            message: format!("unsupported version {} (expected 1)", definition.version),
        });
    }
    if definition.name != requested_name {
        return Err(ThemeError::Invalid {
            name: requested_name.to_string(),
            message: format!("declares name '{}'", definition.name),
        });
    }
    Ok(())
}

fn color(theme: &str, role: &str, name: &str) -> Result<AnsiColor, ThemeError> {
    AnsiColor::from_name(name).ok_or_else(|| ThemeError::Invalid {
        name: theme.to_string(),
        message: format!("palette.{role} uses unknown ANSI color '{name}'"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_every_documented_builtin_theme() {
        for name in ["default", "minimal", "git-focus", "powerline"] {
            load_builtin(name).unwrap_or_else(|error| panic!("{name}: {error}"));
        }
    }

    #[test]
    fn minimal_theme_disables_icons_and_reduces_the_palette() {
        let theme = load_builtin("minimal").expect("minimal theme should load");

        assert_eq!(theme.icons, IconSetting::Never);
        assert_eq!(theme.palette.color("primary"), Some(AnsiColor::White));
        assert_eq!(theme.palette.color("secondary"), Some(AnsiColor::White));
    }

    #[test]
    fn rejects_unknown_theme_names_with_available_choices() {
        let error = load_builtin("missing").expect_err("unknown theme should fail");

        assert!(
            error
                .to_string()
                .contains("default, minimal, git-focus, powerline")
        );
    }

    #[test]
    fn powerline_theme_exposes_an_ascii_fallback() {
        let theme = load_builtin("powerline").expect("powerline theme should load");
        let separator = theme.separator.expect("powerline separator should exist");

        assert_eq!(
            separator.render(false, true, "primary", &theme.palette),
            " | "
        );
    }
}
