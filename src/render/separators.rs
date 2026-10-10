use super::ansi;
use super::palette::Palette;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Separator {
    glyph: String,
    fallback: String,
}

impl Separator {
    pub fn new(glyph: impl Into<String>, fallback: impl Into<String>) -> Self {
        Self {
            glyph: glyph.into(),
            fallback: fallback.into(),
        }
    }

    pub fn render(
        &self,
        enhanced: bool,
        color: bool,
        previous_style: &str,
        palette: &Palette,
    ) -> String {
        if !enhanced {
            return self.fallback.clone();
        }

        ansi::paint(
            &self.glyph,
            ansi::style_for_key(previous_style, palette),
            color,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_ascii_fallback_without_enhanced_font_support() {
        let separator = Separator::new("\u{e0b0}", " | ");
        assert_eq!(
            separator.render(false, true, "primary", &Palette::default()),
            " | "
        );
    }

    #[test]
    fn colors_powerline_glyph_with_the_previous_segment_role() {
        let separator = Separator::new("\u{e0b0}", " | ");
        assert_eq!(
            separator.render(true, true, "success", &Palette::default()),
            "\u{1b}[32m\u{e0b0}\u{1b}[0m"
        );
    }

    #[test]
    fn keeps_powerline_glyph_plain_when_ansi_is_disabled() {
        let separator = Separator::new("\u{e0b0}", " | ");
        assert_eq!(
            separator.render(true, false, "primary", &Palette::default()),
            "\u{e0b0}"
        );
    }
}
