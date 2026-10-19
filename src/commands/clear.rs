use std::io;

use crate::render;

const CLEAR_SCREEN: &str = "\u{1b}[3J\u{1b}[2J\u{1b}[1;1H";

/// Clears the visible screen and scrollback in ANSI-compatible terminals.
pub fn clear() -> io::Result<()> {
    render::stdout(CLEAR_SCREEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_emits_screen_and_scrollback_sequences() {
        let (result, output) = render::capture_output(clear);

        assert!(result.is_ok());
        assert_eq!(output.stdout, "\u{1b}[3J\u{1b}[2J\u{1b}[1;1H");
    }
}
