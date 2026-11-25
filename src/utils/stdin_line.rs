//! Line input from the standard input of the shell, which it shares with the
//! programs it starts.
//!
//! `std::io::stdin` reads ahead into a buffer of the process, so lines meant for a
//! program or for a later `read` would stay there. Every line taken from descriptor
//! 0 goes through this module.

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal};

/// A reader that takes nothing from the standard input past the lines asked of it.
///
/// A terminal gives one line at a time already, and `std` is what decodes the
/// UTF-16 of a Windows console. Anything else is read one byte at a time.
pub fn reader() -> io::Result<Box<dyn BufRead>> {
    let stdin = io::stdin();
    if stdin.is_terminal() {
        return Ok(Box::new(stdin.lock()));
    }
    #[cfg(unix)]
    let owned = std::os::fd::AsFd::as_fd(&stdin).try_clone_to_owned()?;
    #[cfg(windows)]
    let owned = std::os::windows::io::AsHandle::as_handle(&stdin).try_clone_to_owned()?;
    Ok(Box::new(BufReader::with_capacity(1, File::from(owned))))
}

/// Append the next line to `line`, as [`BufRead::read_line`] does.
pub fn read_line(line: &mut String) -> io::Result<usize> {
    reader()?.read_line(line)
}
