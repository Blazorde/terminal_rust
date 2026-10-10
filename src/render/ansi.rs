use std::env;

use super::palette::Palette;

const ESCAPE_PREFIX: &str = "\u{1b}[";
const RESET: &str = "\u{1b}[0m";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnsiColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl AnsiColor {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "black" => Some(Self::Black),
            "red" => Some(Self::Red),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "blue" => Some(Self::Blue),
            "magenta" => Some(Self::Magenta),
            "cyan" => Some(Self::Cyan),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    const fn foreground_code(self) -> u8 {
        match self {
            Self::Black => 30,
            Self::Red => 31,
            Self::Green => 32,
            Self::Yellow => 33,
            Self::Blue => 34,
            Self::Magenta => 35,
            Self::Cyan => 36,
            Self::White => 37,
        }
    }

    const fn background_code(self) -> u8 {
        self.foreground_code() + 10
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnsiStyle {
    pub foreground: Option<AnsiColor>,
    pub background: Option<AnsiColor>,
    pub bold: bool,
    pub dim: bool,
}

impl AnsiStyle {
    pub const fn foreground(color: AnsiColor) -> Self {
        Self {
            foreground: Some(color),
            background: None,
            bold: false,
            dim: false,
        }
    }

    pub const fn with_background(mut self, color: AnsiColor) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }
}

pub fn paint(text: &str, style: AnsiStyle, enabled: bool) -> String {
    if !enabled || text.is_empty() {
        return text.to_string();
    }

    let mut codes = Vec::with_capacity(4);
    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if let Some(color) = style.foreground {
        codes.push(color.foreground_code().to_string());
    }
    if let Some(color) = style.background {
        codes.push(color.background_code().to_string());
    }

    if codes.is_empty() {
        return text.to_string();
    }

    format!("{ESCAPE_PREFIX}{}m{text}{RESET}", codes.join(";"))
}

pub fn style_for_key(style_key: &str, palette: &Palette) -> AnsiStyle {
    if style_key == "inverse" {
        return AnsiStyle::foreground(palette.color("muted").unwrap_or(AnsiColor::White))
            .with_background(palette.color("primary").unwrap_or(AnsiColor::Black));
    }

    let Some(color) = palette.color(style_key) else {
        return AnsiStyle::default();
    };

    let style = AnsiStyle::foreground(color);
    match style_key {
        "primary" | "secondary" | "warning" | "error" => style.bold(),
        "muted" => style.dim(),
        _ => style,
    }
}

pub fn color_enabled(is_terminal: bool) -> bool {
    let ansi_capable = if cfg!(windows) {
        env::var_os("WT_SESSION").is_some()
            || env::var_os("ANSICON").is_some()
            || env::var("ConEmuANSI").is_ok_and(|value| value == "ON")
            || env::var("TERM_PROGRAM").is_ok_and(|value| value == "vscode")
            || env::var("TERM").is_ok_and(|value| value != "dumb")
    } else {
        true
    };

    color_enabled_for(
        is_terminal,
        env::var_os("NO_COLOR").is_some(),
        env::var("TERM").ok().as_deref(),
        ansi_capable,
    )
}

fn color_enabled_for(
    is_terminal: bool,
    no_color: bool,
    term: Option<&str>,
    ansi_capable: bool,
) -> bool {
    is_terminal && !no_color && term != Some("dumb") && ansi_capable
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paints_foreground_background_and_attributes_then_resets() {
        let style = AnsiStyle::foreground(AnsiColor::Cyan)
            .with_background(AnsiColor::Blue)
            .bold()
            .dim();

        assert_eq!(
            paint("prompt", style, true),
            "\u{1b}[1;2;36;44mprompt\u{1b}[0m"
        );
    }

    #[test]
    fn leaves_text_plain_when_color_is_disabled() {
        let style = AnsiStyle::foreground(AnsiColor::Green).bold();

        assert_eq!(paint("main", style, false), "main");
    }

    #[test]
    fn skips_escape_codes_when_style_is_empty() {
        assert_eq!(paint("plain", AnsiStyle::default(), true), "plain");
    }

    #[test]
    fn detects_terminal_color_fallbacks() {
        assert!(color_enabled_for(true, false, Some("xterm-256color"), true));
        assert!(!color_enabled_for(
            false,
            false,
            Some("xterm-256color"),
            true
        ));
        assert!(!color_enabled_for(true, true, Some("xterm-256color"), true));
        assert!(!color_enabled_for(true, false, Some("dumb"), true));
        assert!(!color_enabled_for(true, false, None, false));
    }

    #[test]
    fn exposes_all_portable_basic_colors() {
        let colors = [
            AnsiColor::Black,
            AnsiColor::Red,
            AnsiColor::Green,
            AnsiColor::Yellow,
            AnsiColor::Blue,
            AnsiColor::Magenta,
            AnsiColor::Cyan,
            AnsiColor::White,
        ];

        assert_eq!(
            colors.map(AnsiColor::foreground_code),
            [30, 31, 32, 33, 34, 35, 36, 37]
        );
        assert_eq!(AnsiColor::from_name("magenta"), Some(AnsiColor::Magenta));
        assert_eq!(AnsiColor::from_name("unknown"), None);
    }

    #[test]
    fn maps_semantic_keys_without_exposing_escape_codes_to_segments() {
        let palette = Palette::default();
        assert_eq!(
            style_for_key("warning", &palette),
            AnsiStyle::foreground(AnsiColor::Yellow).bold()
        );
        assert_eq!(style_for_key("unknown", &palette), AnsiStyle::default());
    }

    #[test]
    fn renderer_uses_an_injected_palette() {
        let palette = Palette::new(
            AnsiColor::Blue,
            AnsiColor::White,
            AnsiColor::Cyan,
            AnsiColor::Magenta,
            AnsiColor::Yellow,
            AnsiColor::Black,
        );

        assert_eq!(
            paint("path", style_for_key("primary", &palette), true),
            "\u{1b}[1;34mpath\u{1b}[0m"
        );
    }
}
