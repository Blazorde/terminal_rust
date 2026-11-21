//! Brace expansion, `{a,b}` and `{1..5}` (see ADR-0015).
//!
//! It works on the text of a segment, before substitutions and parsing: a word
//! with a brace group becomes several words, each a copy of the word with one of
//! the alternatives in place. Quotes and escapes are left as they were typed, so
//! the normal parser reads the result.

use crate::app::parser::scan_substitution;

/// The most words one brace group may produce, so that `{1..99999999}` cannot
/// freeze the shell.
const MAX_WORDS: usize = 10_000;

const TOO_MANY: &str = "brace expansion would produce too many words";

/// Expand the brace groups of the unquoted words in `text`.
pub(super) fn expand(text: &str) -> Result<String, String> {
    if !text.contains('{') {
        return Ok(text.to_string());
    }
    let mut output = String::with_capacity(text.len());
    for (word, separator) in split_words(text) {
        if !word.is_empty() {
            output.push_str(&expand_word(&word)?.join(" "));
        }
        output.push_str(&separator);
    }
    Ok(output)
}

/// Step over a `$(...)` that starts at `position`, which is the `$`. Returns the
/// position of its closing parenthesis, or `None` when it is never closed.
fn skip_substitution(characters: &[char], position: usize) -> Option<usize> {
    let mut index = position + 2;
    scan_substitution(
        || {
            let next = characters.get(index).copied();
            index += 1;
            next
        },
        |_| {},
    )
    .ok()
    .map(|()| index - 1)
}

/// The words of `text` (raw, quotes kept) each with the blanks that follow it.
fn split_words(text: &str) -> Vec<(String, String)> {
    let characters: Vec<char> = text.chars().collect();
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    let mut position = 0;
    while position < characters.len() {
        let character = characters[position];
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => {
                quote = None;
                word.push(character);
            }
            (Some('\''), _) => word.push(character),
            (_, '\\') => {
                word.push(character);
                if let Some(&escaped) = characters.get(position + 1) {
                    word.push(escaped);
                    position += 1;
                }
            }
            (Some('"') | None, '$') if characters.get(position + 1) == Some(&'(') => {
                // An unclosed one is reported later, by the parser.
                let end = skip_substitution(&characters, position).unwrap_or(characters.len() - 1);
                word.extend(&characters[position..=end]);
                position = end;
            }
            (None, '\'') => {
                quote = Some('\'');
                word.push(character);
            }
            (None, '"') => {
                quote = Some('"');
                word.push(character);
            }
            (None, blank) if blank.is_whitespace() => {
                let mut separator = String::from(blank);
                while characters
                    .get(position + 1)
                    .is_some_and(|next| next.is_whitespace())
                {
                    position += 1;
                    separator.push(characters[position]);
                }
                words.push((std::mem::take(&mut word), separator));
            }
            (_, _) => word.push(character),
        }
        position += 1;
    }
    words.push((word, String::new()));
    words
}

/// Expand one raw word. A word that carries an operator is left alone: its
/// redirections and pipes are read later, and splitting it would cut them.
fn expand_word(word: &str) -> Result<Vec<String>, String> {
    let characters: Vec<char> = word.chars().collect();
    if find_unquoted(&characters, 0, |c, _| {
        matches!(c, '|' | '<' | '>' | '&' | ';' | '(' | ')')
    })
    .is_some()
    {
        return Ok(vec![word.to_string()]);
    }
    expand_characters(&characters)
}

/// The position of the first character that is not quoted, escaped or inside a
/// `$(...)` or a `${...}` and satisfies `test(character, position)`.
fn find_unquoted(
    word: &[char],
    start: usize,
    mut test: impl FnMut(char, usize) -> bool,
) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut position = start;
    while position < word.len() {
        let character = word[position];
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => quote = None,
            (Some('\''), _) => {}
            (_, '\\') => position += 1,
            (Some('"') | None, '$') if word.get(position + 1) == Some(&'(') => {
                if let Some(end) = skip_substitution(word, position) {
                    position = end;
                }
            }
            (Some('"') | None, '$') if word.get(position + 1) == Some(&'{') => {
                // A `${NAME}` is a variable, not a brace group.
                if let Some(end) = word[position + 2..].iter().position(|&c| c == '}') {
                    position += 2 + end;
                }
            }
            (None, '\'') => quote = Some('\''),
            (None, '"') => quote = Some('"'),
            (None, _) if test(character, position) => return Some(position),
            _ => {}
        }
        position += 1;
    }
    None
}

/// Expand the first group of `word` and, recursively, whatever else it holds.
fn expand_characters(word: &[char]) -> Result<Vec<String>, String> {
    let Some(open) = find_unquoted(word, 0, |c, _| c == '{') else {
        return Ok(vec![word.iter().collect()]);
    };
    let mut depth = 0_usize;
    let close = find_unquoted(word, open, |c, _| match c {
        '{' => {
            depth += 1;
            false
        }
        '}' => {
            depth -= 1;
            depth == 0
        }
        _ => false,
    });
    let inner = close.map(|close| word[open + 1..close].iter().collect::<String>());
    let alternatives = inner
        .as_deref()
        .and_then(|inner| split_alternatives(inner).or_else(|| sequence(inner)));
    let (Some(close), Some(alternatives)) = (close, alternatives) else {
        // Not a group: the brace is an ordinary character.
        let head: String = word[..=open].iter().collect();
        return Ok(expand_characters(&word[open + 1..])?
            .into_iter()
            .map(|rest| format!("{head}{rest}"))
            .collect());
    };

    let prefix: String = word[..open].iter().collect();
    let suffix: String = word[close + 1..].iter().collect();
    let mut results = Vec::new();
    for alternative in alternatives {
        let joined: Vec<char> = format!("{prefix}{alternative}{suffix}").chars().collect();
        results.extend(expand_characters(&joined)?);
        if results.len() > MAX_WORDS {
            return Err(TOO_MANY.to_string());
        }
    }
    Ok(results)
}

/// The alternatives of `{a,b,c}`, split at the commas that are not nested,
/// quoted or escaped. `None` when there is no comma.
fn split_alternatives(inner: &str) -> Option<Vec<String>> {
    let characters: Vec<char> = inner.chars().collect();
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0_usize;
    let mut quote: Option<char> = None;
    let mut position = 0;
    while position < characters.len() {
        let character = characters[position];
        match (quote, character) {
            (Some('\''), '\'') | (Some('"'), '"') => {
                quote = None;
                current.push(character);
            }
            (Some(_), _) => current.push(character),
            (None, '\\') => {
                current.push(character);
                if let Some(&escaped) = characters.get(position + 1) {
                    current.push(escaped);
                    position += 1;
                }
            }
            (None, '$') if characters.get(position + 1) == Some(&'(') => {
                let end = skip_substitution(&characters, position).unwrap_or(characters.len() - 1);
                current.extend(&characters[position..=end]);
                position = end;
            }
            (None, '\'') => {
                quote = Some('\'');
                current.push(character);
            }
            (None, '"') => {
                quote = Some('"');
                current.push(character);
            }
            (None, '{') => {
                depth += 1;
                current.push(character);
            }
            (None, '}') => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            (None, ',') if depth == 0 => parts.push(std::mem::take(&mut current)),
            (None, _) => current.push(character),
        }
        position += 1;
    }
    if parts.is_empty() {
        return None;
    }
    parts.push(current);
    Some(parts)
}

/// `{1..5}`, `{5..1}`, `{a..e}`, `{01..10}` and `{1..10..3}`.
fn sequence(inner: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = inner.split("..").collect();
    let (from, to, step) = match parts.as_slice() {
        [from, to] => (*from, *to, None),
        [from, to, step] => (*from, *to, Some(*step)),
        _ => return None,
    };
    let step = match step {
        Some(text) => i64::try_from(text.parse::<i64>().ok()?.unsigned_abs().max(1)).ok()?,
        None => 1,
    };
    if let (Ok(start), Ok(end)) = (from.parse::<i64>(), to.parse::<i64>()) {
        // A leading zero on either end asks for padding to the longest.
        let width = [from, to]
            .iter()
            .map(|text| text.trim_start_matches('-'))
            .filter(|digits| digits.len() > 1 && digits.starts_with('0'))
            .map(str::len)
            .max()
            .unwrap_or(0);
        return numbers(start, end, step, width);
    }
    let letter = |text: &str| {
        let mut characters = text.chars();
        match (characters.next(), characters.next()) {
            (Some(c), None) if c.is_ascii_alphabetic() => Some(i64::from(u32::from(c))),
            _ => None,
        }
    };
    let (start, end) = (letter(from)?, letter(to)?);
    let codes = numbers(start, end, step, 0)?;
    Some(
        codes
            .iter()
            .filter_map(|code| code.parse::<u8>().ok())
            .map(|byte| char::from(byte).to_string())
            .collect(),
    )
}

/// The values from `start` to `end` by `step`, padded with zeros to `width`.
/// `None` when there would be too many.
fn numbers(start: i64, end: i64, step: i64, width: usize) -> Option<Vec<String>> {
    let direction: i64 = if end >= start { 1 } else { -1 };
    let mut values = Vec::new();
    let mut value = start;
    while (direction > 0 && value <= end) || (direction < 0 && value >= end) {
        if values.len() >= MAX_WORDS {
            return None;
        }
        values.push(if width > 0 {
            let sign = if value < 0 { "-" } else { "" };
            format!("{sign}{:0>width$}", value.unsigned_abs())
        } else {
            value.to_string()
        });
        value = value.checked_add(direction * step)?;
    }
    Some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> String {
        expand(text).expect("the text should expand")
    }

    #[test]
    fn a_comma_group_gives_one_word_per_alternative() {
        assert_eq!(words("echo {a,b,c}"), "echo a b c");
        assert_eq!(words("echo file{.txt,.md}"), "echo file.txt file.md");
        assert_eq!(words("cp f{,.bak}"), "cp f f.bak");
        assert_eq!(
            words("mkdir -p proj/{src,tests}/x"),
            "mkdir -p proj/src/x proj/tests/x"
        );
    }

    #[test]
    fn several_groups_give_every_combination_in_order() {
        assert_eq!(words("echo {a,b}{1,2}"), "echo a1 a2 b1 b2");
        assert_eq!(words("echo x{a,b}y{1,2}z"), "echo xay1z xay2z xby1z xby2z");
    }

    #[test]
    fn groups_nest() {
        assert_eq!(words("echo {a,{b,c}d}"), "echo a bd cd");
        assert_eq!(words("echo {{a,b},c}"), "echo a b c");
    }

    #[test]
    fn sequences_of_numbers_and_letters() {
        assert_eq!(words("echo {1..5}"), "echo 1 2 3 4 5");
        assert_eq!(words("echo {5..1}"), "echo 5 4 3 2 1");
        assert_eq!(words("echo {a..e}"), "echo a b c d e");
        assert_eq!(words("echo {e..a..2}"), "echo e c a");
        assert_eq!(words("echo {1..10..3}"), "echo 1 4 7 10");
        assert_eq!(words("echo {-2..2}"), "echo -2 -1 0 1 2");
        assert_eq!(words("echo {01..03}"), "echo 01 02 03");
        assert_eq!(
            words("echo img{8..10}.png"),
            "echo img8.png img9.png img10.png"
        );
    }

    #[test]
    fn a_group_that_is_not_one_stays_as_written() {
        for text in [
            "echo {}",
            "echo {a}",
            "echo {a b}",
            "echo {",
            "echo a}",
            "echo {1..}",
            "echo {a..bb}",
        ] {
            assert_eq!(words(text), text, "{text}");
        }
        assert_eq!(words("echo {a} {b,c}"), "echo {a} b c");
    }

    #[test]
    fn quoting_and_escaping_keep_braces_literal() {
        assert_eq!(words("echo '{a,b}'"), "echo '{a,b}'");
        assert_eq!(words("echo \"{a,b}\""), "echo \"{a,b}\"");
        assert_eq!(words("echo \\{a,b}"), "echo \\{a,b}");
        assert_eq!(words("echo {a,'b,c'}"), "echo a 'b,c'");
        assert_eq!(words("echo {\"x,y\",z}"), "echo \"x,y\" z");
    }

    #[test]
    fn a_variable_in_braces_is_not_a_group() {
        assert_eq!(words("echo ${HOME}"), "echo ${HOME}");
        assert_eq!(words("echo ${HOME}/{a,b}"), "echo ${HOME}/a ${HOME}/b");
    }

    #[test]
    fn a_substitution_is_opaque() {
        assert_eq!(words("echo $(echo {a,b})"), "echo $(echo {a,b})");
        assert_eq!(words("echo x{$(echo a,b),c}"), "echo x$(echo a,b) xc");
    }

    #[test]
    fn a_word_with_an_operator_is_left_alone() {
        assert_eq!(words("echo {a,b}>out"), "echo {a,b}>out");
        assert_eq!(words("cat x | echo {a,b}"), "cat x | echo a b");
    }

    #[test]
    fn spacing_between_words_is_kept() {
        assert_eq!(words("echo   {a,b}    end"), "echo   a b    end");
    }

    #[test]
    fn too_many_words_are_refused() {
        assert_eq!(
            expand("echo {1..99999999}"),
            Ok("echo {1..99999999}".to_string())
        );
        assert!(expand("echo {1..100}{1..100}{1..2}").is_err());
    }
}
