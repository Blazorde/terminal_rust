//! The standard input of an in-process command inside a pipeline.
//!
//! A program reads its stdin from the process; a built-in command runs inside
//! the shell and has no stream of its own. The pipeline stores what the previous
//! stage produced here for the duration of the stage, and a command that reads
//! its input (`cat` without a file) takes it.

use std::cell::RefCell;

thread_local! {
    static INPUT: RefCell<Option<Vec<u8>>> = const { RefCell::new(None) };
}

/// Make `input` available to the command about to run: `None` when it has no
/// input from a pipeline or a redirection.
pub(crate) fn provide(input: Option<Vec<u8>>) {
    INPUT.with(|slot| *slot.borrow_mut() = input);
}

/// Whether the current command has an input from a pipeline or a redirection.
pub(crate) fn is_provided() -> bool {
    INPUT.with(|slot| slot.borrow().is_some())
}

/// Take the input of the current command. It can only be read once.
pub(crate) fn take() -> Option<Vec<u8>> {
    INPUT.with(|slot| slot.borrow_mut().take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_input_can_be_taken_once() {
        provide(Some(b"data".to_vec()));

        assert_eq!(take(), Some(b"data".to_vec()));
        assert_eq!(take(), None);
    }

    #[test]
    fn the_presence_of_an_input_can_be_checked_without_taking_it() {
        provide(None);
        assert!(!is_provided());
        provide(Some(Vec::new()));
        assert!(is_provided());
        assert_eq!(take(), Some(Vec::new()));
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
}
