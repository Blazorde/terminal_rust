//! `$NA<Tab>` and `${NA<Tab>`: the names of the variables.

use rustyline::completion::Pair;

/// For a word that ends inside a variable reference (`$HO`, `${HO`), where the
/// name starts in the word, the part typed so far, and whether it is `${...}`.
pub(super) fn variable_reference(word: &str) -> Option<(usize, &str, bool)> {
    let dollar = word.rfind('$')?;
    let after = &word[dollar + 1..];
    let (offset, prefix, braced) = match after.strip_prefix('{') {
        Some(rest) => (dollar + 2, rest, true),
        None => (dollar + 1, after, false),
    };
    prefix
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
        .then_some((offset, prefix, braced))
}

pub(super) fn variable_completions(names: &[String], prefix: &str, braced: bool) -> Vec<Pair> {
    names
        .iter()
        .filter(|name| name.starts_with(prefix))
        .map(|name| Pair {
            display: name.clone(),
            replacement: if braced {
                format!("{name}}}")
            } else {
                name.clone()
            },
        })
        .collect()
}
