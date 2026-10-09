use std::io;

use crate::render;

const CLEAR_SCREEN: &str = "\u{1b}[2J\u{1b}[H";

/// Clears an ANSI-compatible terminal and moves the cursor to the top-left.
pub fn clear() -> io::Result<()> {
    render::stdout(CLEAR_SCREEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_sequence_erases_screen_and_moves_cursor_home() {
        assert_eq!(CLEAR_SCREEN, "\u{1b}[2J\u{1b}[H");
    }
}
