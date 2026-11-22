//! The operators of `${name…}` that work on the value (see ADR-0030): removing a prefix or a
//! suffix that matches a pattern, replacing a match, taking a part, and changing the case.

use crate::utils::glob;

/// Apply the operator in `operation` (what follows the name inside the braces) to `value`.
/// `Ok(None)` when `operation` is not one of these operators. `expand` turns the pattern or
/// the replacement text into the text to use (variables expanded).
pub(super) fn apply(
    operation: &str,
    value: &str,
    expand: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Option<String>, String> {
    let mut characters = operation.chars();
    let Some(first) = characters.next() else {
        return Ok(None);
    };
    let rest = characters.as_str();
    let text = match first {
        '#' => remove_prefix(
            value,
            &pattern(rest.strip_prefix('#').unwrap_or(rest), expand)?,
            rest.starts_with('#'),
        ),
        '%' => remove_suffix(
            value,
            &pattern(rest.strip_prefix('%').unwrap_or(rest), expand)?,
            rest.starts_with('%'),
        ),
        '/' => replace(value, rest, expand)?,
        '^' => change_case(value, rest.starts_with('^'), true),
        ',' => change_case(value, rest.starts_with(','), false),
        ':' if is_substring(rest) => substring(value, rest, expand)?,
        _ => return Ok(None),
    };
    Ok(Some(text))
}

fn pattern(text: &str, expand: &dyn Fn(&str) -> Result<String, String>) -> Result<String, String> {
    expand(text)
}

/// The prefix lengths (in bytes, at character boundaries) of `value`, from 0 to its length.
fn boundaries(value: &str) -> impl DoubleEndedIterator<Item = usize> + '_ {
    value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
}

/// `${v#pattern}` (the shortest match) and `${v##pattern}` (the longest) from the start.
fn remove_prefix(value: &str, pattern: &str, longest: bool) -> String {
    if pattern.is_empty() {
        return value.to_string();
    }
    let matching = |end: &usize| glob::matches_text(pattern, &value[..*end]);
    let end = if longest {
        boundaries(value).rev().find(matching)
    } else {
        boundaries(value).find(matching)
    };
    end.map_or_else(|| value.to_string(), |end| value[end..].to_string())
}

/// `${v%pattern}` (the shortest match) and `${v%%pattern}` (the longest) from the end.
fn remove_suffix(value: &str, pattern: &str, longest: bool) -> String {
    if pattern.is_empty() {
        return value.to_string();
    }
    let matching = |start: &usize| glob::matches_text(pattern, &value[*start..]);
    let start = if longest {
        boundaries(value).find(matching)
    } else {
        boundaries(value).rev().find(matching)
    };
    start.map_or_else(|| value.to_string(), |start| value[..start].to_string())
}

/// `${v/pattern/text}` the first match, `${v//pattern/text}` every match, `${v/#pattern/text}`
/// at the start and `${v/%pattern/text}` at the end. The longest match wins.
fn replace(
    value: &str,
    rest: &str,
    expand: &dyn Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let (mode, rest) = match rest.chars().next() {
        Some('/') => ('a', &rest[1..]),
        Some('#') => ('s', &rest[1..]),
        Some('%') => ('e', &rest[1..]),
        _ => ('f', rest),
    };
    let (pattern_text, replacement_text) = split_unescaped(rest);
    let pattern = expand(pattern_text)?;
    let replacement = expand(replacement_text)?;
    if pattern.is_empty() {
        return Ok(value.to_string());
    }
    let longest_from = |start: usize| {
        boundaries(value)
            .rev()
            .filter(|end| *end >= start)
            .find(|end| glob::matches_text(&pattern, &value[start..*end]))
    };
    match mode {
        's' => Ok(match longest_from(0) {
            Some(end) => format!("{replacement}{}", &value[end..]),
            None => value.to_string(),
        }),
        'e' => Ok(
            match boundaries(value).find(|start| glob::matches_text(&pattern, &value[*start..])) {
                Some(start) => format!("{}{replacement}", &value[..start]),
                None => value.to_string(),
            },
        ),
        _ => {
            let mut output = String::new();
            let mut position = 0;
            let starts: Vec<usize> = boundaries(value).collect();
            let mut index = 0;
            while index < starts.len() {
                let start = starts[index];
                if start < position {
                    index += 1;
                    continue;
                }
                // A match of nothing is not replaced.
                if let Some(end) = longest_from(start).filter(|end| *end > start) {
                    output.push_str(&value[position..start]);
                    output.push_str(&replacement);
                    position = end;
                    if mode == 'f' {
                        break;
                    }
                }
                index += 1;
            }
            output.push_str(&value[position..]);
            Ok(output)
        }
    }
}

/// Split `a/b` at the first `/` that is not escaped; no `/` leaves the replacement empty.
fn split_unescaped(text: &str) -> (&str, &str) {
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        match (escaped, character) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '/') => return (&text[..index], &text[index + 1..]),
            _ => {}
        }
    }
    (text, "")
}

/// `^` and `^^` make upper case (the first character, or all), `,` and `,,` lower case.
fn change_case(value: &str, all: bool, upper: bool) -> String {
    let convert = |text: &str| {
        if upper {
            text.to_uppercase()
        } else {
            text.to_lowercase()
        }
    };
    if all {
        return convert(value);
    }
    let mut characters = value.chars();
    match characters.next() {
        Some(first) => convert(&first.to_string()) + characters.as_str(),
        None => String::new(),
    }
}

/// Whether what follows `:` is an offset (`2`, ` -3`, `(-3)`, `1:2`) and not `-default`.
pub(super) fn is_substring(rest: &str) -> bool {
    rest.starts_with(|character: char| {
        character.is_ascii_alphanumeric() || matches!(character, '(' | ' ' | '$' | '_')
    })
}

/// `${v:offset}` and `${v:offset:length}`, counted in characters.
fn substring(
    value: &str,
    rest: &str,
    expand: &dyn Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let characters: Vec<char> = value.chars().collect();
    let (start, end) = range(characters.len(), rest, expand)?;
    Ok(characters[start..end].iter().collect())
}

/// The part `offset[:length]` of `total` items, as `start..end`: a negative offset counts from
/// the end, and a negative length stops that many items before the end.
pub(super) fn range(
    total: usize,
    rest: &str,
    expand: &dyn Fn(&str) -> Result<String, String>,
) -> Result<(usize, usize), String> {
    let (offset_text, length_text) = match rest.split_once(':') {
        Some((offset, length)) => (offset, Some(length)),
        None => (rest, None),
    };
    let number = |text: &str| -> Result<i64, String> {
        // A bare name is a variable, as in an arithmetic expression: `${v:n}`.
        let name = text.trim();
        let text = if crate::utils::shell_env::is_valid_name(name) {
            expand(&format!("${name}"))?
        } else {
            expand(text)?
        };
        text.trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("{text}: substring expression expected"))
    };
    let total = i64::try_from(total).unwrap_or(i64::MAX);
    let offset = number(offset_text)?;
    let start = if offset < 0 { total + offset } else { offset };
    if start < 0 || start > total {
        return Ok((0, 0));
    }
    let end = match length_text {
        None => total,
        Some(length) => {
            let length = number(length)?;
            if length < 0 {
                let end = total + length;
                if end < start {
                    return Err(format!("{length}: substring expression < 0"));
                }
                end
            } else {
                (start + length).min(total)
            }
        }
    };
    Ok((start as usize, end as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(operation: &str, value: &str) -> String {
        apply(operation, value, &|text| Ok(text.to_string()))
            .expect("applies")
            .expect("is an operator")
    }

    #[test]
    fn prefixes_and_suffixes_go_shortest_or_longest() {
        assert_eq!(run("#*/", "a/b/c"), "b/c");
        assert_eq!(run("##*/", "a/b/c"), "c");
        assert_eq!(run("%/*", "a/b/c"), "a/b");
        assert_eq!(run("%%/*", "a/b/c"), "a");
        assert_eq!(run("%.txt", "name.txt"), "name");
        assert_eq!(run("#hel", "hello"), "lo");
        assert_eq!(run("#x", "hello"), "hello");
        assert_eq!(run("%?", "été"), "ét");
        assert_eq!(run("#", "abc"), "abc");
    }

    #[test]
    fn a_match_is_replaced_once_everywhere_or_at_an_end() {
        assert_eq!(run("/l/L", "hello"), "heLlo");
        assert_eq!(run("//l/L", "hello"), "heLLo");
        assert_eq!(run("//l/", "hello"), "heo");
        assert_eq!(run("//l", "hello"), "heo");
        assert_eq!(run("/#h/J", "hello"), "Jello");
        assert_eq!(run("/#e/J", "hello"), "hello");
        assert_eq!(run("/%o/!", "hello"), "hell!");
        assert_eq!(run("/l*o/X", "hello"), "heX");
        assert_eq!(run("//[aeiou]/_", "education"), "_d_c_t__n");
        assert_eq!(run("/x/y", "hello"), "hello");
        // A `/` that is escaped belongs to the pattern, the first other one ends it.
        assert_eq!(split_unescaped("a\\/b/c"), ("a\\/b", "c"));
        assert_eq!(split_unescaped("abc"), ("abc", ""));
        assert_eq!(run("//a/-", "banana"), "b-n-n-");
    }

    #[test]
    fn a_part_is_taken_by_offset_and_length() {
        assert_eq!(run(":1", "hello"), "ello");
        assert_eq!(run(":1:3", "hello"), "ell");
        assert_eq!(run(": -3", "hello"), "llo");
        assert_eq!(run(":(-3):2", "hello"), "ll");
        assert_eq!(run(":0:-1", "hello"), "hell");
        assert_eq!(run(":9", "hello"), "");
        assert_eq!(run(":2:99", "hello"), "llo");
        assert_eq!(run(":1:2", "été!"), "té");
        // A bare name is a variable (the test expander does not know any): an error, not text.
        let error = apply(":x", "a", &|text| Ok(text.to_string())).unwrap_err();
        assert!(error.contains("substring expression expected"), "{error}");
    }

    #[test]
    fn the_case_changes() {
        assert_eq!(run("^^", "hello"), "HELLO");
        assert_eq!(run("^", "hello"), "Hello");
        assert_eq!(run(",,", "HELLO"), "hello");
        assert_eq!(run(",", "HELLO"), "hELLO");
        assert_eq!(run("^^", "été"), "ÉTÉ");
        assert_eq!(run("^", ""), "");
    }

    #[test]
    fn other_operations_are_not_ours() {
        for operation in ["", "-default", ":-default", ":+x", ":?x", "=x", "x"] {
            assert_eq!(
                apply(operation, "v", &|text| Ok(text.to_string())),
                Ok(None),
                "{operation:?}"
            );
        }
    }
}
