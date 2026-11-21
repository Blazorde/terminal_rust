//! File name patterns: `*`, `?` and `[...]` (see ADR-0012).
//!
//! A pattern is matched one path segment at a time, so `*` never crosses a `/`.
//! A segment that starts with `.` is only matched by a pattern that starts with
//! `.` as well, which keeps hidden files out of `*`.

use std::fs;
use std::path::Path;

/// One element of a compiled pattern segment.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Literal(char),
    /// `*`
    Star,
    /// `?`
    Any,
    /// `[...]`: the ranges (a single character is a range of one), and whether
    /// the class is negated.
    Class {
        negated: bool,
        ranges: Vec<(char, char)>,
    },
}

/// Compile one segment. `\x` is the literal `x`, and a `[` that is never closed
/// is an ordinary character.
fn compile(pattern: &str) -> Vec<Token> {
    let characters: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        match characters[index] {
            '\\' if index + 1 < characters.len() => {
                tokens.push(Token::Literal(characters[index + 1]));
                index += 2;
            }
            '*' => {
                tokens.push(Token::Star);
                index += 1;
            }
            '?' => {
                tokens.push(Token::Any);
                index += 1;
            }
            '[' => match compile_class(&characters, index) {
                Some((token, next)) => {
                    tokens.push(token);
                    index = next;
                }
                None => {
                    tokens.push(Token::Literal('['));
                    index += 1;
                }
            },
            other => {
                tokens.push(Token::Literal(other));
                index += 1;
            }
        }
    }
    tokens
}

/// Read the class that starts at `characters[start]` (`[`): the token and the
/// position after the `]`, or `None` if it is not closed.
fn compile_class(characters: &[char], start: usize) -> Option<(Token, usize)> {
    let mut index = start + 1;
    let negated = matches!(characters.get(index), Some('!' | '^'));
    if negated {
        index += 1;
    }
    let first = index;
    let mut ranges = Vec::new();
    while let Some(&current) = characters.get(index) {
        // A `]` right after the opening is part of the class.
        if current == ']' && index > first {
            return Some((Token::Class { negated, ranges }, index + 1));
        }
        if characters.get(index + 1) == Some(&'-')
            && characters.get(index + 2).is_some_and(|&end| end != ']')
        {
            ranges.push((current, characters[index + 2]));
            index += 3;
        } else {
            ranges.push((current, current));
            index += 1;
        }
    }
    None
}

fn is_wildcard(token: &Token) -> bool {
    !matches!(token, Token::Literal(_))
}

/// Whether `text` has a `*`, `?` or `[...]` that is not escaped.
#[cfg(test)]
pub fn is_pattern(text: &str) -> bool {
    compile(text).iter().any(is_wildcard)
}

/// The text a segment stands for when it holds no wildcard.
fn literal(tokens: &[Token]) -> String {
    tokens
        .iter()
        .filter_map(|token| match token {
            Token::Literal(character) => Some(*character),
            _ => None,
        })
        .collect()
}

/// Match one name against one pattern segment.
#[cfg(test)]
pub fn matches(pattern: &str, name: &str) -> bool {
    match_tokens(&compile(pattern), name)
}

fn match_tokens(tokens: &[Token], name: &str) -> bool {
    // Hidden names are only matched by a pattern that names the dot itself.
    if name.starts_with('.') && tokens.first() != Some(&Token::Literal('.')) {
        return false;
    }
    let name: Vec<char> = name.chars().collect();
    let (mut t, mut n) = (0, 0);
    // Where to resume after the last `*`: its token position and the name
    // position it has consumed up to.
    let mut star: Option<(usize, usize)> = None;

    while n < name.len() {
        let advanced = match tokens.get(t) {
            Some(Token::Star) => {
                star = Some((t, n));
                t += 1;
                continue;
            }
            Some(Token::Any) => true,
            Some(Token::Literal(character)) => *character == name[n],
            Some(Token::Class { negated, ranges }) => {
                let inside = ranges
                    .iter()
                    .any(|&(low, high)| low <= name[n] && name[n] <= high);
                inside != *negated
            }
            None => false,
        };
        if advanced {
            t += 1;
            n += 1;
        } else if let Some((star_t, star_n)) = star {
            // Mismatch: let the last `*` swallow one more character.
            t = star_t + 1;
            n = star_n + 1;
            star = Some((star_t, star_n + 1));
        } else {
            return false;
        }
    }
    tokens[t..].iter().all(|token| *token == Token::Star)
}

/// The paths that match `pattern`, sorted. Empty when nothing matches.
///
/// Only the segments that hold a pattern read a directory, so `src/*/mod.rs`
/// lists `src` and checks `mod.rs` in each match.
pub fn expand(pattern: &str) -> Vec<String> {
    let absolute = pattern.starts_with('/');
    let segments: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let mut found = Vec::new();
    let base = if absolute {
        "/".to_string()
    } else {
        String::new()
    };
    walk(&base, &segments, &mut found);
    found.sort();
    found
}

fn walk(base: &str, segments: &[&str], found: &mut Vec<String>) {
    let Some((segment, rest)) = segments.split_first() else {
        if !base.is_empty() {
            found.push(base.to_string());
        }
        return;
    };
    let join = |name: &str| {
        if base.is_empty() || base.ends_with('/') {
            format!("{base}{name}")
        } else {
            format!("{base}/{name}")
        }
    };

    let tokens = compile(segment);
    if !tokens.iter().any(is_wildcard) {
        let candidate = join(&literal(&tokens));
        // The last segment must exist; a middle one is checked by the next step.
        if rest.is_empty() {
            if fs::symlink_metadata(&candidate).is_ok() {
                found.push(candidate);
            }
        } else {
            walk(&candidate, rest, found);
        }
        return;
    }

    let directory = if base.is_empty() { "." } else { base };
    let Ok(entries) = fs::read_dir(Path::new(directory)) else {
        return;
    };
    let names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| match_tokens(&tokens, name))
        .collect();
    for name in names {
        let candidate = join(&name);
        // A middle segment must be a directory to be followed.
        if rest.is_empty() || Path::new(&candidate).is_dir() {
            walk(&candidate, rest, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn m(pattern: &str, name: &str) -> bool {
        matches(pattern, name)
    }

    #[test]
    fn star_matches_any_run_of_characters() {
        assert!(m("*", "anything"));
        assert!(m("*.rs", "main.rs"));
        assert!(m("a*c", "abbbc"));
        assert!(m("a*c", "ac"));
        assert!(m("*a*b*", "xxaxxbxx"));
        assert!(!m("*.rs", "main.rc"));
        assert!(!m("a*c", "abd"));
        assert!(m("**", "x"));
    }

    #[test]
    fn star_backtracks() {
        assert!(m("*ab", "aab"));
        assert!(m("*aab", "aaaab"));
        assert!(!m("*aab", "aaaa"));
        assert!(m("a*a*a", "aaa"));
    }

    #[test]
    fn question_mark_matches_exactly_one_character() {
        assert!(m("?", "a"));
        assert!(!m("?", ""));
        assert!(!m("?", "ab"));
        assert!(m("f?le", "file"));
        assert!(m("?é", "aé"));
    }

    #[test]
    fn classes_match_sets_ranges_and_negations() {
        assert!(m("[abc]", "b"));
        assert!(!m("[abc]", "d"));
        assert!(m("[a-c]x", "bx"));
        assert!(!m("[a-c]x", "dx"));
        assert!(m("[!a-c]", "z"));
        assert!(!m("[!a-c]", "a"));
        assert!(m("[^a]", "b"));
        assert!(m("[]a]", "]"));
        assert!(m("file[0-9].txt", "file7.txt"));
        assert!(m("[a-]", "-"));
    }

    #[test]
    fn an_unclosed_bracket_is_an_ordinary_character() {
        assert!(m("[abc", "[abc"));
        assert!(m("a[", "a["));
        assert!(!m("[abc", "a"));
    }

    #[test]
    fn hidden_names_need_an_explicit_dot() {
        assert!(!m("*", ".hidden"));
        assert!(!m("?hidden", ".hidden"));
        assert!(!m("[.]hidden", ".hidden"));
        assert!(m(".*", ".hidden"));
        assert!(m(".hid*", ".hidden"));
    }

    #[test]
    fn an_escaped_character_is_literal() {
        assert!(m("a\\*b", "a*b"));
        assert!(!m("a\\*b", "axb"));
        assert!(m("\\[x]", "[x]"));
        assert!(m("\\?", "?"));
        assert!(m("a\\\\b", "a\\b"));
        assert!(!is_pattern("a\\*b"));
        assert!(is_pattern("a\\**"));
    }

    #[test]
    fn a_pattern_is_recognised() {
        assert!(is_pattern("*.rs"));
        assert!(is_pattern("a?"));
        assert!(is_pattern("[ab]"));
        assert!(!is_pattern("plain.txt"));
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("terminal-rust-glob-{name}-{nonce}"));
        fs::create_dir_all(root.join("src/sub")).expect("fixture");
        for file in [
            "a.rs",
            "b.rs",
            "c.txt",
            ".hidden.rs",
            "src/x.rs",
            "src/sub/y.rs",
        ] {
            fs::write(root.join(file), "").expect("fixture file");
        }
        root
    }

    fn absolute(root: &Path, pattern: &str) -> Vec<String> {
        let prefix = format!("{}/", root.display());
        expand(&format!("{prefix}{pattern}"))
            .into_iter()
            .map(|path| path.strip_prefix(&prefix).unwrap_or(&path).to_string())
            .collect()
    }

    #[test]
    fn expansion_lists_sorted_matches_without_hidden_files() {
        let root = fixture("sorted");

        assert_eq!(absolute(&root, "*.rs"), ["a.rs", "b.rs"]);
        assert_eq!(absolute(&root, "*"), ["a.rs", "b.rs", "c.txt", "src"]);
        assert_eq!(absolute(&root, ".*.rs"), [".hidden.rs"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_pattern_can_cross_directories_one_segment_at_a_time() {
        let root = fixture("segments");

        assert_eq!(absolute(&root, "src/*.rs"), ["src/x.rs"]);
        assert_eq!(absolute(&root, "*/*.rs"), ["src/x.rs"]);
        assert_eq!(absolute(&root, "s*/*/y.rs"), ["src/sub/y.rs"]);
        assert_eq!(absolute(&root, "src/sub/?.rs"), ["src/sub/y.rs"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn nothing_matching_gives_an_empty_list() {
        let root = fixture("nothing");

        assert!(absolute(&root, "*.zip").is_empty());
        assert!(absolute(&root, "missing/*.rs").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_relative_pattern_stays_relative() {
        let _guard = crate::test_support::cwd_lock().lock().expect("lock");
        let root = fixture("relative");
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("enter fixture");

        let found = expand("*.rs");

        std::env::set_current_dir(previous).expect("restore cwd");
        assert_eq!(found, ["a.rs", "b.rs"]);
        fs::remove_dir_all(root).expect("cleanup");
    }
}
