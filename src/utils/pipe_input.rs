//! The standard input of an in-process command inside a pipeline.
//!
//! A program reads its stdin from the process; a built-in command runs inside
//! the shell and has no stream of its own. The pipeline stores what the previous
//! stage produced here for the duration of the stage, and a command that reads
//! its input takes it: `cat` takes everything that is left, `read` one line at a
//! time, so that `while read line; do …; done < file` can read the file line by
//! line (see ADR-0022).

use std::cell::RefCell;

/// What was provided, and how much of it has been read.
struct Buffer {
    bytes: Vec<u8>,
    position: usize,
}

thread_local! {
    static INPUT: RefCell<Option<Buffer>> = const { RefCell::new(None) };
}

/// Make `input` available to the command about to run: `None` when it has no
/// input from a pipeline or a redirection. The pipeline uses [`scoped`]; tests set
/// the input directly.
#[cfg(test)]
pub(crate) fn provide(input: Option<Vec<u8>>) {
    INPUT.with(|slot| {
        *slot.borrow_mut() = input.map(|bytes| Buffer { bytes, position: 0 });
    });
}

/// Whether the current command has an input from a pipeline or a redirection,
/// even one that has been read to its end.
pub(crate) fn is_provided() -> bool {
    INPUT.with(|slot| slot.borrow().is_some())
}

/// Take what is left of the input of the current command. The input stays, read
/// to its end: a command that comes after finds nothing, and does not fall back
/// to the terminal.
pub(crate) fn take() -> Option<Vec<u8>> {
    INPUT.with(|slot| {
        slot.borrow_mut().as_mut().map(|buffer| {
            let rest = buffer.bytes[buffer.position..].to_vec();
            buffer.position = buffer.bytes.len();
            rest
        })
    })
}

/// The result of reading one line of the input.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Line {
    /// There is no input from a pipeline or a redirection: the terminal is the input.
    NoInput,
    /// The input is over.
    End,
    /// The next line, without its line ending.
    Text(String),
}

/// Read the next line of the input, and leave the rest for the next call.
pub(crate) fn next_line() -> Line {
    INPUT.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(buffer) = slot.as_mut() else {
            return Line::NoInput;
        };
        let rest = &buffer.bytes[buffer.position..];
        if rest.is_empty() {
            return Line::End;
        }
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(rest.len(), |newline| newline + 1);
        let line = String::from_utf8_lossy(&rest[..end])
            .trim_end_matches(['\n', '\r'])
            .to_string();
        buffer.position += end;
        Line::Text(line)
    })
}

/// Run `operation` with `input` as the input, then put back what was there. With
/// `None` the input that is already there stays: a command inside a loop that
/// reads a file shares the file with the loop.
pub(crate) fn scoped<T>(input: Option<Vec<u8>>, operation: impl FnOnce() -> T) -> T {
    let Some(bytes) = input else {
        return operation();
    };
    let outer = INPUT.with(|slot| slot.borrow_mut().replace(Buffer { bytes, position: 0 }));
    let result = operation();
    INPUT.with(|slot| *slot.borrow_mut() = outer);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_input_can_be_taken_once_and_leaves_nothing_after() {
        provide(Some(b"data".to_vec()));

        assert_eq!(take(), Some(b"data".to_vec()));
        // It is still an input, read to its end.
        assert_eq!(take(), Some(Vec::new()));
        assert_eq!(next_line(), Line::End);
        provide(None);
        assert_eq!(take(), None);
    }

    #[test]
    fn the_presence_of_an_input_can_be_checked_without_taking_it() {
        provide(None);
        assert!(!is_provided());
        provide(Some(Vec::new()));
        assert!(is_provided());
        assert_eq!(take(), Some(Vec::new()));
        assert!(is_provided());
        provide(None);
        assert!(!is_provided());
    }

    #[test]
    fn providing_nothing_clears_the_previous_input() {
        provide(Some(b"old".to_vec()));
        provide(None);

        assert_eq!(take(), None);
    }

    #[test]
    fn an_empty_input_is_still_an_input() {
        provide(Some(Vec::new()));

        assert_eq!(take(), Some(Vec::new()));
    }

    #[test]
    fn lines_are_read_one_at_a_time_and_the_rest_stays() {
        provide(Some(b"one\r\ntwo\nthree".to_vec()));

        assert_eq!(next_line(), Line::Text("one".to_string()));
        assert_eq!(next_line(), Line::Text("two".to_string()));
        // What is left goes to a command that reads everything.
        assert_eq!(take(), Some(b"three".to_vec()));
        assert_eq!(next_line(), Line::End);
        provide(None);
    }

    #[test]
    fn the_end_of_the_input_is_not_the_absence_of_one() {
        provide(Some(b"only\n".to_vec()));

        assert_eq!(next_line(), Line::Text("only".to_string()));
        assert_eq!(next_line(), Line::End);
        assert_eq!(next_line(), Line::End);
        assert!(is_provided());
        provide(None);
        assert_eq!(next_line(), Line::NoInput);
        provide(Some(Vec::new()));
        assert_eq!(next_line(), Line::End);
        provide(None);
    }

    #[test]
    fn a_blank_line_is_a_line() {
        provide(Some(b"\n\nx\n".to_vec()));

        assert_eq!(next_line(), Line::Text(String::new()));
        assert_eq!(next_line(), Line::Text(String::new()));
        assert_eq!(next_line(), Line::Text("x".to_string()));
        assert_eq!(next_line(), Line::End);
        provide(None);
    }

    #[test]
    fn a_scoped_input_replaces_the_outer_one_and_gives_it_back() {
        provide(Some(b"outer1\nouter2\n".to_vec()));
        assert_eq!(next_line(), Line::Text("outer1".to_string()));

        let inner = scoped(Some(b"inner\n".to_vec()), next_line);
        assert_eq!(inner, Line::Text("inner".to_string()));

        // The outer input is where it was left.
        assert_eq!(next_line(), Line::Text("outer2".to_string()));
        provide(None);
    }

    #[test]
    fn without_an_input_of_its_own_a_scope_shares_the_outer_one() {
        provide(Some(b"shared\n".to_vec()));

        let seen = scoped(None, next_line);

        assert_eq!(seen, Line::Text("shared".to_string()));
        assert_eq!(next_line(), Line::End);
        provide(None);
        assert_eq!(scoped(None, next_line), Line::NoInput);
    }
}
