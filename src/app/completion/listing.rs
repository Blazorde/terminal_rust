//! What is handed to the line editor: a list that is safe to show and to insert,
//! and short enough to be listed.

use rustyline::completion::Pair;

use crate::render::printable::printable;

/// The most candidates offered at once. The line editor cannot list more than
/// 65 535 of them, and nobody reads that many.
pub(super) const MAX_CANDIDATES: usize = 2_000;

/// Drop the candidates that cannot be typed, remove duplicates, bound the list
/// and sort it by what is shown.
pub(super) fn finish(mut candidates: Vec<Pair>) -> Vec<Pair> {
    // A name with a control character would act on the terminal once inserted.
    candidates.retain(|candidate| printable(&candidate.replacement) == candidate.replacement);
    candidates.sort_by(|left, right| left.replacement.cmp(&right.replacement));
    candidates.dedup_by(|left, right| left.replacement == right.replacement);
    // The editor inserts what all candidates share. In a sorted list that is what
    // the first and the last share, so keeping the last one keeps it right.
    if candidates.len() > MAX_CANDIDATES {
        let last = candidates.pop();
        candidates.truncate(MAX_CANDIDATES - 1);
        candidates.extend(last);
    }
    for candidate in &mut candidates {
        candidate.display = printable(&candidate.display).into_owned();
    }
    candidates.sort_by(|left, right| left.display.cmp(&right.display));
    candidates
}

/// What the candidates share may stop right after a `\` that escapes the next
/// character (`a\ b` and `a\$c` share `a\`): that `\` is left out. On Windows a
/// `\` is a path separator and stays.
pub(super) fn without_dangling_escape(text: &str) -> &str {
    let escapes = text.len() - text.trim_end_matches('\\').len();
    if cfg!(windows) || escapes.is_multiple_of(2) {
        text
    } else {
        &text[..text.len() - 1]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyline::completion::longest_common_prefix;

    fn pair(text: &str) -> Pair {
        Pair {
            display: text.to_string(),
            replacement: text.to_string(),
        }
    }

    fn replacements(pairs: &[Pair]) -> Vec<&str> {
        pairs.iter().map(|pair| pair.replacement.as_str()).collect()
    }

    #[test]
    fn candidates_are_sorted_and_offered_once() {
        let list = finish(vec![pair("b"), pair("a"), pair("b")]);

        assert_eq!(replacements(&list), ["a", "b"]);
    }

    #[test]
    fn a_name_with_a_control_character_is_not_offered() {
        let list = finish(vec![
            pair("safe"),
            pair("x\u{1b}]0;title\u{7}"),
            pair("two\nlines"),
            pair("re\u{202e}versed"),
        ]);

        assert_eq!(replacements(&list), ["safe"]);
    }

    #[test]
    fn what_is_shown_has_no_control_character() {
        let list = finish(vec![Pair {
            display: "a\u{1b}[31m".to_string(),
            replacement: "a".to_string(),
        }]);

        assert_eq!(list[0].display, "a?[31m");
    }

    #[test]
    fn a_long_list_is_bounded_and_keeps_the_common_prefix() {
        let mut names: Vec<Pair> = (0..MAX_CANDIDATES + 500)
            .map(|number| pair(&format!("file-a{number:05}")))
            .collect();
        // Sorts last and shares less with the others.
        names.push(pair("file-z"));

        let list = finish(names);

        assert_eq!(list.len(), MAX_CANDIDATES);
        assert_eq!(longest_common_prefix(&list), Some("file-"));
    }

    #[test]
    fn a_dangling_escape_is_left_out_where_backslash_escapes() {
        let expected = if cfg!(windows) { "a\\" } else { "a" };

        assert_eq!(without_dangling_escape("a\\"), expected);
        assert_eq!(without_dangling_escape("a\\\\"), "a\\\\");
        assert_eq!(without_dangling_escape("a\\ b"), "a\\ b");
        assert_eq!(without_dangling_escape(""), "");
    }
}
