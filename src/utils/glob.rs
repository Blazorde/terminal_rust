//! File name patterns: `*`, `?`, `[...]` and `**` (see ADR-0012 and ADR-0020).
//!
//! A pattern is matched one path segment at a time, so `*` never crosses a `/`.
//! A segment that starts with `.` is only matched by a pattern that starts with
//! `.` as well, which keeps hidden files out of `*`. A segment that is exactly
//! `**` stands for any number of directories, including none.

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

/// Match `text` against a whole pattern, as `case` does: `*` and `?` also match
/// `/` and a leading `.`, since this is not a file name.
pub fn matches_text(pattern: &str, text: &str) -> bool {
    match_run(&compile(pattern), text)
}

/// `text` with every character that is special in a pattern escaped, so that it
/// matches only itself.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '*' | '?' | '[' | '\\') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn match_tokens(tokens: &[Token], name: &str) -> bool {
    // Hidden names are only matched by a pattern that names the dot itself.
    if name.starts_with('.') && tokens.first() != Some(&Token::Literal('.')) {
        return false;
    }
    match_run(tokens, name)
}

fn match_run(tokens: &[Token], name: &str) -> bool {
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

/// The deepest directory a `**` goes down to, and the most paths one pattern may
/// give: a `**` started in a huge tree must not run for minutes or fill the memory.
const MAX_DEPTH: usize = 32;
const MAX_MATCHES: usize = 50_000;

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

fn join(base: &str, name: &str) -> String {
    if base.is_empty() || base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

/// The names in `base` (the current directory when empty) that are not hidden,
/// with whether each is a real directory. A link to a directory is not followed,
/// so a `**` cannot loop.
fn visible_entries(base: &str) -> Vec<(String, bool)> {
    let directory = if base.is_empty() { "." } else { base };
    let Ok(entries) = fs::read_dir(Path::new(directory)) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let is_directory = entry.file_type().ok()?.is_dir();
            (!name.starts_with('.')).then_some((name, is_directory))
        })
        .collect()
}

/// `**` followed by `rest`: `rest` in `base` and in every directory below it.
/// With nothing after it, everything below `base`.
fn globstar(base: &str, rest: &[&str], found: &mut Vec<String>, depth: usize) {
    if found.len() >= MAX_MATCHES {
        return;
    }
    if rest.is_empty() {
        for (name, is_directory) in visible_entries(base) {
            let path = join(base, &name);
            found.push(path.clone());
            if is_directory && depth < MAX_DEPTH && found.len() < MAX_MATCHES {
                globstar(&path, rest, found, depth + 1);
            }
        }
        return;
    }
    // No directory at all, then one more level at a time.
    walk(base, rest, found);
    if depth >= MAX_DEPTH {
        return;
    }
    for (name, is_directory) in visible_entries(base) {
        if is_directory {
            globstar(&join(base, &name), rest, found, depth + 1);
        }
    }
}

fn walk(base: &str, segments: &[&str], found: &mut Vec<String>) {
    let Some((segment, rest)) = segments.split_first() else {
        if !base.is_empty() {
            found.push(base.to_string());
        }
        return;
    };
    let join = |name: &str| join(base, name);
    if *segment == "**" {
        globstar(base, rest, found, 0);
        return;
    }

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
    fn a_text_is_matched_whole_and_without_the_file_name_rules() {
        // Unlike a file name, a text may start with a dot and contain slashes.
        assert!(matches_text("*", ".hidden"));
        assert!(matches_text("*", "a/b/c"));
        assert!(matches_text("?", "."));
        assert!(matches_text("*.txt", ".a.txt"));
        assert!(matches_text("a/*", "a/b/c"));
        assert!(matches_text("[a-c]*", "bcd"));
        assert!(matches_text("", ""));
        // The whole text must match.
        assert!(!matches_text("a", "ab"));
        assert!(!matches_text("ab", "a"));
        assert!(!matches_text("", "x"));
        assert!(!matches_text("[!a]", "a"));
    }

    #[test]
    fn an_escaped_text_matches_only_itself() {
        for text in ["*", "a?b", "[x]", "back\\slash", "plain", ""] {
            assert!(matches_text(&escape(text), text), "{text:?}");
        }
        assert!(!matches_text(&escape("*"), "anything"));
        assert!(!matches_text(&escape("a?b"), "axb"));
        assert!(!matches_text(&escape("[x]"), "x"));
        assert_eq!(escape("a*b?[c\\"), "a\\*b\\?\\[c\\\\");
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
    fn a_double_star_segment_goes_through_every_directory() {
        let root = fixture("globstar");

        // Zero directories and every depth, sorted, without hidden files.
        assert_eq!(
            absolute(&root, "**/*.rs"),
            ["a.rs", "b.rs", "src/sub/y.rs", "src/x.rs"]
        );
        assert_eq!(absolute(&root, "**/y.rs"), ["src/sub/y.rs"]);
        assert_eq!(absolute(&root, "src/**/*.rs"), ["src/sub/y.rs", "src/x.rs"]);
        // In the middle, with something before and after it.
        assert_eq!(absolute(&root, "src/**/y.rs"), ["src/sub/y.rs"]);
        assert_eq!(absolute(&root, "s*/**/*.rs"), ["src/sub/y.rs", "src/x.rs"]);
        // A pattern after it still applies to the whole name.
        assert!(absolute(&root, "**/*.zip").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_trailing_double_star_lists_everything_below() {
        let root = fixture("trailing");

        assert_eq!(
            absolute(&root, "**"),
            [
                "a.rs",
                "b.rs",
                "c.txt",
                "src",
                "src/sub",
                "src/sub/y.rs",
                "src/x.rs"
            ]
        );
        assert_eq!(
            absolute(&root, "src/**"),
            ["src/sub", "src/sub/y.rs", "src/x.rs"]
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_double_star_skips_hidden_directories_and_does_not_follow_links() {
        let root = fixture("hiddenstar");
        fs::create_dir_all(root.join(".git/objects")).expect("fixture");
        fs::write(root.join(".git/objects/z.rs"), "").expect("fixture");
        #[cfg(unix)]
        {
            // A link back to the root would make the walk endless if followed.
            std::os::unix::fs::symlink(&root, root.join("src/loop")).expect("symlink");
        }

        let found = absolute(&root, "**/*.rs");
        assert!(!found.iter().any(|path| path.contains(".git")), "{found:?}");
        assert!(
            !found.iter().any(|path| path.contains("loop/")),
            "{found:?}"
        );
        assert_eq!(found, ["a.rs", "b.rs", "src/sub/y.rs", "src/x.rs"]);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn an_escaped_or_embedded_double_star_is_not_a_globstar() {
        let root = fixture("embedded");
        // `a**` is an ordinary star pattern: one level only.
        assert_eq!(absolute(&root, "s**"), ["src"]);
        assert_eq!(absolute(&root, "*/x**"), ["src/x.rs"]);
        // Escaped stars are literal text.
        assert!(absolute(&root, "\\*\\*/*.rs").is_empty());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_double_star_stops_at_the_depth_limit() {
        let root = std::env::temp_dir().join(format!(
            "terminal-rust-glob-deep-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let mut deep = root.clone();
        for level in 0..(MAX_DEPTH + 6) {
            deep = deep.join(format!("d{level}"));
        }
        fs::create_dir_all(&deep).expect("fixture");
        fs::write(deep.join("far.txt"), "").expect("fixture");
        fs::write(root.join("d0").join("near.txt"), "").expect("fixture");

        let found = absolute(&root, "**/*.txt");
        assert_eq!(
            found,
            ["d0/near.txt"],
            "the file below the limit is not reached"
        );
        // A trailing `**` has the same limit.
        let everything = absolute(&root, "**");
        assert!(everything.iter().any(|path| path.ends_with("near.txt")));
        assert!(
            !everything.iter().any(|path| path.ends_with("far.txt")),
            "the file below the limit is not reached"
        );
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
