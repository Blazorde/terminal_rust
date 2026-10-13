use std::env;

use crate::prompt::segment::SegmentIcon;

const ICON_MODE_ENV: &str = "TERMINAL_RUST_ICONS";
const NERD_FONT_ENV: &str = "TERMINAL_RUST_NERD_FONT";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IconMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl IconMode {
    pub fn from_environment() -> Self {
        match env::var(ICON_MODE_ENV).as_deref() {
            Ok("always") => Self::Always,
            Ok("never") => Self::Never,
            _ => Self::Auto,
        }
    }

    pub fn enabled(self, is_terminal: bool) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::Auto => {
                is_terminal
                    && env::var(NERD_FONT_ENV)
                        .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes"))
            }
        }
    }
}

pub fn decorate(value: &str, icon: Option<&SegmentIcon>, icons_enabled: bool) -> String {
    let Some(icon) = icon else {
        return value.to_string();
    };

    if icons_enabled && let Some(glyph) = glyph(&icon.name) {
        return format!("{glyph} {value}");
    }

    format!("{}{value}", icon.fallback)
}

fn glyph(name: &str) -> Option<&'static str> {
    match name {
        "folder" => Some("\u{f07b}"),
        "git_branch" => Some("\u{e0a0}"),
        "error" => Some("\u{f071}"),
        "user" => Some("\u{f007}"),
        "host" => Some("\u{f109}"),
        "status" => Some("\u{f058}"),
        "runtime" => Some("\u{e795}"),
        "duration" => Some("\u{f252}"),
        "time" => Some("\u{f017}"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_and_never_modes_are_deterministic() {
        assert!(IconMode::Always.enabled(false));
        assert!(!IconMode::Never.enabled(true));
    }

    #[test]
    fn renders_nerd_font_icon_when_enabled() {
        let icon = SegmentIcon::new("git_branch", "git:");

        assert_eq!(decorate("main*", Some(&icon), true), "\u{e0a0} main*");
    }

    #[test]
    fn renders_ascii_fallback_when_icons_are_disabled_or_unknown() {
        let git = SegmentIcon::new("git_branch", "git:");
        let custom = SegmentIcon::new("custom", "custom:");

        assert_eq!(decorate("main", Some(&git), false), "git:main");
        assert_eq!(decorate("value", Some(&custom), true), "custom:value");
        assert_eq!(decorate("plain", None, true), "plain");
    }

    #[test]
    fn central_registry_contains_planned_prompt_icons() {
        for name in [
            "folder",
            "git_branch",
            "error",
            "user",
            "host",
            "status",
            "runtime",
            "duration",
            "time",
        ] {
            assert!(glyph(name).is_some(), "missing {name} icon");
        }
    }
}
