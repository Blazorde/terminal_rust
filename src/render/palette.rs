use super::ansi::AnsiColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    primary: AnsiColor,
    secondary: AnsiColor,
    success: AnsiColor,
    warning: AnsiColor,
    error: AnsiColor,
    muted: AnsiColor,
}

impl Palette {
    pub const fn new(
        primary: AnsiColor,
        secondary: AnsiColor,
        success: AnsiColor,
        warning: AnsiColor,
        error: AnsiColor,
        muted: AnsiColor,
    ) -> Self {
        Self {
            primary,
            secondary,
            success,
            warning,
            error,
            muted,
        }
    }

    pub fn color(&self, name: &str) -> Option<AnsiColor> {
        match name {
            "primary" => Some(self.primary),
            "secondary" => Some(self.secondary),
            "success" => Some(self.success),
            "warning" => Some(self.warning),
            "error" => Some(self.error),
            "muted" => Some(self.muted),
            _ => None,
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        let named = |name, fallback| AnsiColor::from_name(name).unwrap_or(fallback);

        Self::new(
            named("cyan", AnsiColor::Cyan),
            named("magenta", AnsiColor::Magenta),
            named("green", AnsiColor::Green),
            named("yellow", AnsiColor::Yellow),
            named("red", AnsiColor::Red),
            named("white", AnsiColor::White),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_palette_exposes_semantic_colors() {
        let palette = Palette::default();

        assert_eq!(palette.color("primary"), Some(AnsiColor::Cyan));
        assert_eq!(palette.color("success"), Some(AnsiColor::Green));
        assert_eq!(palette.color("warning"), Some(AnsiColor::Yellow));
        assert_eq!(palette.color("unknown"), None);
    }

    #[test]
    fn custom_palette_can_replace_colors_without_prompt_changes() {
        let palette = Palette::new(
            AnsiColor::Blue,
            AnsiColor::White,
            AnsiColor::Cyan,
            AnsiColor::Magenta,
            AnsiColor::Yellow,
            AnsiColor::Black,
        );

        assert_eq!(palette.color("primary"), Some(AnsiColor::Blue));
        assert_eq!(palette.color("muted"), Some(AnsiColor::Black));
    }
}
