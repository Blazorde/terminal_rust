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
    use crate::app::parser::{Connector, Redirect, split_chain, split_stages};

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

    fn segments(input: &str) -> Vec<(Connector, String)> {
        split_chain(input)
            .expect("the line should split")
            .into_iter()
            .map(|segment| (segment.connector, segment.text.trim().to_string()))
            .collect()
    }

    fn stages(input: &str) -> Vec<(String, Vec<Redirect>)> {
        split_stages(input)
            .expect("the segment should split")
            .into_iter()
            .map(|stage| (stage.text.trim().to_string(), stage.redirects))
            .collect()
    }

    #[test]
    fn a_comparison_in_an_arithmetic_command_does_not_split_the_chain() {
        assert_eq!(
            segments("(( a < b )) && x"),
            vec![
                (Connector::Then, "(( a < b ))".to_string()),
                (Connector::And, "x".to_string()),
            ]
        );
        assert_eq!(
            segments("(( a > b )) || y; z"),
            vec![
                (Connector::Then, "(( a > b ))".to_string()),
                (Connector::Or, "y".to_string()),
                (Connector::Then, "z".to_string()),
            ]
        );
        assert_eq!(
            segments("(( x && y )) || (( p || q ))"),
            vec![
                (Connector::Then, "(( x && y ))".to_string()),
                (Connector::Or, "(( p || q ))".to_string()),
            ]
        );
    }

    #[test]
    fn a_pipe_or_ampersand_in_the_body_is_not_an_operator() {
        for text in ["(( a | b ))", "(( x && y ))", "(( x & y ))", "(( 2 > 1 ))"] {
            assert_eq!(
                segments(text),
                vec![(Connector::Then, text.to_string())],
                "{text}"
            );
            assert_eq!(stages(text), vec![(text.to_string(), vec![])], "{text}");
        }
    }

    #[test]
    fn a_redirection_and_a_pipe_after_the_command_are_still_operators() {
        let target = Redirect::Stdout {
            target: "f".into(),
            append: false,
        };
        assert_eq!(
            stages("(( 1 )) > f | cat"),
            vec![
                ("(( 1 ))".to_string(), vec![target]),
                ("cat".to_string(), vec![]),
            ]
        );
        assert_eq!(
            stages("echo hi | (( a < b ))"),
            vec![
                ("echo hi".to_string(), vec![]),
                ("(( a < b ))".to_string(), vec![]),
            ]
        );
    }

    #[test]
    fn a_background_marker_after_the_command_ends_the_segment() {
        let parsed = split_chain("(( 7 )) &").expect("the line should split");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].text.trim(), "(( 7 ))");
        assert!(parsed[0].background);
    }

    #[test]
    fn nested_subshells_are_split_like_their_spaced_form() {
        for (tight, spaced) in [
            ("((echo a); echo b)", "( (echo a); echo b)"),
            ("((echo a); (echo b))", "( (echo a); (echo b))"),
            ("((echo a) | cat)", "( (echo a) | cat)"),
        ] {
            let tight_segments = split_chain(tight).expect("tight form should split");
            let spaced_segments = split_chain(spaced).expect("spaced form should split");
            let words = |parsed: &[crate::app::parser::Segment]| {
                parsed
                    .iter()
                    .map(|segment| {
                        segment
                            .text
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
            };
            assert_eq!(words(&tight_segments), words(&spaced_segments), "{tight}");
            assert!(words(&tight_segments).len() > 1, "{tight}");
        }
    }

    #[test]
    fn an_arithmetic_command_is_recognized_only_where_a_command_starts() {
        assert_eq!(
            segments("echo ((1)) > f"),
            vec![(Connector::Then, "echo ((1)) > f".to_string())]
        );
        assert_eq!(
            segments("if (( a < b )); then echo y; fi"),
            vec![
                (Connector::Then, "if (( a < b ))".to_string()),
                (Connector::Then, "then echo y".to_string()),
                (Connector::Then, "fi".to_string()),
            ]
        );
    }

    #[test]
    fn array_assignments_are_untouched() {
        assert_eq!(
            segments("x=(1 2); echo $x"),
            vec![
                (Connector::Then, "x=(1 2)".to_string()),
                (Connector::Then, "echo $x".to_string()),
            ]
        );
        assert_eq!(
            segments("a[1]=((3)) && echo ok"),
            vec![
                (Connector::Then, "a[1]=((3))".to_string()),
                (Connector::And, "echo ok".to_string()),
            ]
        );
    }
}
