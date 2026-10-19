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
    fn clear_also_purges_scrollback() {
        assert_eq!(CLEAR_SCREEN, "\u{1b}[3J\u{1b}[2J\u{1b}[1;1H");
    }
}
