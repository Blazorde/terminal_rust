//! The path in a word as typed, and how to write a name back into it. The rules
//! are those of the parser. Nothing here looks at the disk.

/// Characters a `\` protects where it is not a path separator.
const ESCAPABLE: [char; 7] = ['\\', '\'', '"', '$', '*', '?', '['];

/// Characters that only quotes protect.
const QUOTED_ONLY: [char; 11] = [';', '|', '&', '<', '>', '(', ')', '{', '}', '`', '!'];

#[derive(Debug, Eq, PartialEq)]
pub(super) struct PathWord<'a> {
    /// Byte index in the word where the replacement starts.
    pub(super) start: usize,
    /// The text from `start` to the name, kept as typed.
    pub(super) kept: &'a str,
    /// The directory typed, without quotes and escapes; empty or ending with a
    /// separator.
    pub(super) directory: String,
    /// The start of the name typed, without quotes and escapes.
    pub(super) name: String,
    /// The quote the name is written in.
    pub(super) quote: Option<char>,
    /// The last separator typed.
    pub(super) separator: Option<char>,
    /// Whether the directory starts with a `~` that stands for the home.
    pub(super) home: bool,
}

fn is_escape(next: Option<char>, quote: Option<char>, windows: bool) -> bool {
    let Some(next) = next else {
        return false;
    };
    match quote {
        None => {
            next.is_whitespace()
                || (ESCAPABLE.contains(&next) && !(windows && matches!(next, '*' | '?' | '[')))
        }
        Some('"') => matches!(next, '"' | '\\' | '$'),
        Some(_) => false,
    }
}

/// Split `word` into the directory typed and the start of a name. A path may
/// follow a `=`, as in `--output=dir/fi`.
pub(super) fn split_path(word: &str, windows: bool) -> PathWord<'_> {
    let mut quote = None;
    let mut value = String::new();
    // Where the path, then its last name, start; the length of `value` before
    // that name, and the quote open there.
    let (mut path_start, mut name_start, mut directory_length) = (0, 0, 0);
    let mut name_quote = None;
    let mut separator = None;
    let mut characters = word.char_indices().peekable();

    while let Some((at, character)) = characters.next() {
        let next = characters.peek().map(|(_, next)| *next);
        match character {
            '\\' if is_escape(next, quote, windows) => {
                value.extend(characters.next().map(|(_, escaped)| escaped));
                continue;
            }
            '\'' | '"' if quote.is_none() => {
                quote = Some(character);
                continue;
            }
            _ if quote == Some(character) => {
                quote = None;
                continue;
            }
            '=' if quote.is_none() => {
                value.clear();
                path_start = at + 1;
                (name_start, directory_length) = (at + 1, 0);
                (name_quote, separator) = (None, None);
                continue;
            }
            _ => value.push(character),
        }
        if character == '/' || (windows && character == '\\') {
            separator = Some(character);
            name_start = at + 1;
            directory_length = value.len();
            name_quote = quote;
        }
    }

    // A quote that opens the name, or the whole path, stays as it is.
    if let Some(first @ ('\'' | '"')) = word[name_start..].chars().next()
        && name_quote.is_none()
    {
        name_quote = Some(first);
        name_start += 1;
    }
    let leading_quote = word[path_start..].starts_with(['\'', '"']);
    let start = path_start + usize::from(leading_quote);
    let name = value.split_off(directory_length);
    PathWord {
        start,
        kept: &word[start..name_start],
        home: word[path_start..].starts_with('~') && !value.is_empty(),
        directory: value,
        name,
        quote: name_quote,
        separator,
    }
}

fn needs_quotes(character: char, windows: bool) -> bool {
    character.is_whitespace()
        || QUOTED_ONLY.contains(&character)
        || (ESCAPABLE.contains(&character) && !(windows && character == '\\'))
}

/// `name` written so that the parser reads it back, inside `quote` when one is
/// open. `None` when it cannot be written there.
pub(super) fn quoted(name: &str, quote: Option<char>, windows: bool) -> Option<String> {
    match quote {
        Some('"') => {
            // A backquote starts a substitution even between double quotes.
            if name.contains('`') {
                return None;
            }
            let mut written = String::with_capacity(name.len());
            for character in name.chars() {
                if matches!(character, '"' | '$') || (!windows && character == '\\') {
                    written.push('\\');
                }
                written.push(character);
            }
            Some(written)
        }
        Some(_) => Some(name.replace('\'', "'\\''")),
        None => {
            let special = |character| needs_quotes(character, windows);
            // `#` starts a comment and `~` is the home at the start of a word.
            if !name.contains(special) && !name.starts_with(['#', '~']) {
                return Some(name.to_string());
            }
            let escapes_are_enough = !windows
                && !name.contains(|character| QUOTED_ONLY.contains(&character))
                && !name.starts_with(['#', '~']);
            if !escapes_are_enough {
                return Some(format!("'{}'", name.replace('\'', "'\\''")));
            }
            let mut written = String::with_capacity(name.len() * 2);
            for character in name.chars() {
                if special(character) {
                    written.push('\\');
                }
                written.push(character);
            }
            Some(written)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::parser::parse_words;

    /// `(start, kept, directory, name, quote)`.
    fn split(word: &str, windows: bool) -> (usize, &str, String, String, Option<char>) {
        let path = split_path(word, windows);
        (path.start, path.kept, path.directory, path.name, path.quote)
    }

    fn owned(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn a_word_is_split_into_a_directory_and_a_name() {
        assert_eq!(split("fi", false), (0, "", owned(""), owned("fi"), None));
        assert_eq!(
            split("a/b/fi", false),
            (0, "a/b/", owned("a/b/"), owned("fi"), None)
        );
        assert_eq!(
            split("my\\ dir/a\\ fi", false),
            (0, "my\\ dir/", owned("my dir/"), owned("a fi"), None)
        );
        assert_eq!(
            split("\"my dir/fi", false),
            (1, "my dir/", owned("my dir/"), owned("fi"), Some('"'))
        );
        assert_eq!(
            split("'fi", false),
            (1, "", owned(""), owned("fi"), Some('\''))
        );
        assert_eq!(
            split("dir/\"a fi", false),
            (0, "dir/\"", owned("dir/"), owned("a fi"), Some('"'))
        );
        assert_eq!(
            split("'a;b'/fi", false),
            (1, "a;b'/", owned("a;b/"), owned("fi"), None)
        );
        assert_eq!(
            split("--out=dir/fi", false),
            (6, "dir/", owned("dir/"), owned("fi"), None)
        );
        assert_eq!(
            split("a\\b\\fi", true),
            (0, "a\\b\\", owned("a\\b\\"), owned("fi"), None)
        );
        assert!(split_path("~/fi", false).home);
        assert!(!split_path("~fi", false).home);
        assert!(!split_path("\"~/fi", false).home);
    }

    const NAMES: [&str; 12] = [
        "plain.txt",
        "a b",
        "it's",
        "say \"hi\"",
        "$HOME",
        "a;b|c&d",
        "(1) {x,y} [z]",
        "#note",
        "~",
        "what?*",
        "x<y>z",
        "\u{e9}t\u{e9} 1",
    ];

    /// What the parser reads back from `text`.
    fn read_back(text: &str) -> Vec<String> {
        parse_words(text, &|_| Some("EXPANDED".to_string()))
            .expect("the quoting should parse")
            .into_iter()
            .map(|word| word.text)
            .collect()
    }

    #[test]
    fn the_parser_reads_a_written_name_back() {
        let windows = cfg!(windows);
        for name in NAMES {
            for quote in [None, Some('"'), Some('\'')] {
                let written = quoted(name, quote, windows).expect("writable");
                let text = match quote {
                    Some(quote) => format!("{quote}{written}{quote}"),
                    None => written,
                };

                assert_eq!(read_back(&text), [name], "{text}");
            }
        }
    }

    #[test]
    fn a_name_is_escaped_only_when_needed() {
        assert_eq!(quoted("a.txt", None, false), Some(owned("a.txt")));
        assert_eq!(quoted("a b", None, false), Some(owned("a\\ b")));
        assert_eq!(quoted("a b", None, true), Some(owned("'a b'")));
        assert_eq!(quoted("a;b", None, false), Some(owned("'a;b'")));
        assert_eq!(quoted("a`b", Some('"'), false), None);
    }
}
