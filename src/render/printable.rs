//! Make text that comes from outside safe to show in a terminal.
//!
//! A file or directory name, a path or a plugin description can hold control
//! characters. Printed as they are, an escape sequence in a folder called
//! `x<ESC>]0;title<BEL>y` sets the window title, rewrites what is on screen or
//! throws off the width the line editor computes. Every such text is passed
//! through [`printable`] before it is shown.

use std::borrow::Cow;

const STAND_IN: char = '?';

/// True for the characters a terminal could act on, or that change how the text
/// around them is laid out: C0 and C1 controls, DEL, and the bidirectional
/// overrides and isolates that make a name read differently from what it is.
fn is_unsafe(character: char) -> bool {
    character.is_control() || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Replace every control character, newline and tab included, by `?`.
///
/// Meant for names and paths, which are one line of text.
pub fn printable(text: &str) -> Cow<'_, str> {
    replace(text, is_unsafe)
}

/// Like [`printable`] but keeps newlines and tabs, for text that may span
/// several lines.
pub fn printable_multiline(text: &str) -> Cow<'_, str> {
    replace(text, |character| {
        is_unsafe(character) && character != '\n' && character != '\t'
    })
}

fn replace(text: &str, should_replace: impl Fn(char) -> bool) -> Cow<'_, str> {
    if !text.chars().any(&should_replace) {
        return Cow::Borrowed(text);
    }
    Cow::Owned(
        text.chars()
            .map(|character| {
                if should_replace(character) {
                    STAND_IN
                } else {
                    character
                }
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_text_is_returned_without_a_copy() {
        for text in ["", "file.txt", "dossier é à 日本語", "a b\u{a0}c"] {
            assert!(matches!(printable(text), Cow::Borrowed(_)), "{text:?}");
            assert_eq!(printable(text), text);
        }
    }

    #[test]
    fn escape_sequences_are_neutralized() {
        let hostile = "x\u{1b}]0;PWNED\u{7}y";

        assert_eq!(printable(hostile), "x?]0;PWNED?y");
        assert_eq!(printable("evil\u{1b}[31mRED"), "evil?[31mRED");
    }

    #[test]
    fn every_c0_c1_and_del_character_is_replaced() {
        for code in (0u32..0x20).chain(0x7f..=0x9f) {
            let character = char::from_u32(code).expect("code point should be valid");
            let text = format!("a{character}b");

            assert_eq!(printable(&text), "a?b", "U+{code:04X}");
        }
    }

    #[test]
    fn the_c1_csi_and_osc_introducers_are_replaced() {
        assert_eq!(printable("a\u{9b}31mb"), "a?31mb");
        assert_eq!(printable("a\u{9d}0;titleb"), "a?0;titleb");
    }

    #[test]
    fn bidirectional_overrides_that_disguise_a_name_are_replaced() {
        assert_eq!(printable("report\u{202e}gnp.exe"), "report?gnp.exe");
        assert_eq!(printable("a\u{2066}b\u{2069}c"), "a?b?c");
    }

    #[test]
    fn a_newline_or_tab_in_a_name_is_replaced_but_kept_in_multiline_text() {
        assert_eq!(printable("a\nb\tc"), "a?b?c");
        assert_eq!(printable_multiline("a\nb\tc"), "a\nb\tc");
        assert_eq!(printable_multiline("a\u{1b}b\nc"), "a?b\nc");
    }

    #[test]
    fn a_carriage_return_cannot_overwrite_the_line() {
        assert_eq!(printable("safe\rfake"), "safe?fake");
        assert_eq!(printable_multiline("safe\rfake"), "safe?fake");
    }
}
