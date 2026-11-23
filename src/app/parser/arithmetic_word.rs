//! Where a `(( expression ))` command ends.
//!
//! `((` opens an arithmetic command only when the `)` that closes the first level is
//! directly followed by another `)`. Otherwise `((echo a); echo b)` stays two nested
//! subshells, as in bash.

/// The length in characters of the `(( … ))` command at the start of `chars`, which must
/// begin with `((`. `None` when the text is not an arithmetic command.
pub(crate) fn arithmetic_len(mut chars: impl Iterator<Item = char>) -> Option<usize> {
    if chars.next() != Some('(') || chars.next() != Some('(') {
        return None;
    }
    let mut length = 2;
    let mut depth = 0_usize;
    let mut quote: Option<char> = None;
    while let Some(character) = chars.next() {
        length += 1;
        match (quote, character) {
            (Some(open), close) if open == close => quote = None,
            (Some('"'), '\\') | (None, '\\') => {
                chars.next()?;
                length += 1;
            }
            (Some(_), _) => {}
            (None, '\'' | '"') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') if depth > 0 => depth -= 1,
            (None, ')') => {
                return (chars.next() == Some(')')).then_some(length + 1);
            }
            (None, _) => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::arithmetic_len;

    fn len(text: &str) -> Option<usize> {
        arithmetic_len(text.chars())
    }

    #[test]
    fn covers_the_whole_command() {
        assert_eq!(len("((1+2))"), Some(7));
        assert_eq!(len("(( ))"), Some(5));
        assert_eq!(len("(())"), Some(4));
        assert_eq!(len("(( (1+2)*3 )) > f"), Some(13));
    }

    #[test]
    fn nested_subshells_are_not_arithmetic() {
        assert_eq!(len("((echo a); echo b)"), None);
        assert_eq!(len("((echo a); (echo b))"), None);
        assert_eq!(len("((1+2"), None);
        assert_eq!(len("(echo a)"), None);
    }

    #[test]
    fn quotes_and_escapes_hide_parentheses() {
        assert_eq!(len("(( \"a)\" ))"), Some(10));
        assert_eq!(len("(( ')' ))"), Some(9));
        assert_eq!(len("(( \\) ))"), Some(8));
    }
}
