#[derive(Clone, Copy, Eq, PartialEq)]
enum Quote {
    Single,
    Double,
}

/// Look up the value of a variable name (`?` is the last status).
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// One word of a command line. `pattern` is set when it holds an unquoted `*`,
/// `?` or `[`: it is the text to match against file names, in which every other
/// `*`, `?`, `[` and `\` is escaped with a backslash.
#[derive(Debug, Eq, PartialEq)]
pub struct Word {
    pub text: String,
    pub pattern: Option<String>,
}

/// The pattern for a word: `text` with the characters that were not typed as
/// wildcards (quoted, escaped, or from a variable) made literal.
fn pattern_text(text: &str, wildcards: &[usize]) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    for (offset, character) in text.char_indices() {
        let literal_wildcard = matches!(character, '*' | '?' | '[') && !wildcards.contains(&offset);
        if literal_wildcard || (character == '\\' && !cfg!(windows)) {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    pattern
}

/// Split a command line into words, expanding variables and `~`, without
/// matching patterns against files.
pub fn parse_with(input: &str, lookup: Lookup) -> Result<Vec<String>, String> {
    parse_words(input, lookup).map(|words| words.into_iter().map(|word| word.text).collect())
}

/// Split a command line into arguments and replace each pattern by the sorted
/// names it matches. A pattern that matches nothing stays as written, as in
/// POSIX shells. Quoted or escaped characters are never patterns, and neither is
/// the value of a variable.
pub fn parse_args(input: &str, lookup: Lookup) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    for word in parse_words(input, lookup)? {
        if let Some(pattern) = &word.pattern {
            let matches = crate::utils::glob::expand(pattern);
            if !matches.is_empty() {
                args.extend(matches);
                continue;
            }
        }
        args.push(word.text);
    }
    Ok(args)
}

fn finish_word(text: String, wildcards: &mut Vec<usize>) -> Word {
    let pattern = (!wildcards.is_empty()).then(|| pattern_text(&text, wildcards));
    wildcards.clear();
    Word { text, pattern }
}

pub fn parse_words(input: &str, lookup: Lookup) -> Result<Vec<Word>, String> {
    let mut args: Vec<Word> = Vec::new();
    // Byte offsets in `current` of the wildcards that were typed unquoted.
    let mut wildcards: Vec<usize> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<Quote> = None;
    let mut started_argument = false;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => quote = None,
            (Some(Quote::Single), character) => current.push(character),
            (Some(Quote::Double), '\\') if matches!(characters.peek(), Some('"' | '\\' | '$')) => {
                if let Some(escaped) = characters.next() {
                    current.push(escaped);
                }
            }
            (Some(Quote::Double), '$') => {
                expand_variable(&mut characters, lookup, &mut current)?;
            }
            (Some(Quote::Double), character) => current.push(character),
            (None, '\'') => {
                quote = Some(Quote::Single);
                started_argument = true;
            }
            (None, '"') => {
                quote = Some(Quote::Double);
                started_argument = true;
            }
            (None, '\\')
                if characters.peek().is_some_and(|next| {
                    next.is_whitespace()
                        || matches!(next, '\\' | '\'' | '"' | '$')
                        // Where `\` is not a path separator it also protects patterns.
                        || (!cfg!(windows) && matches!(next, '*' | '?' | '['))
                }) =>
            {
                if let Some(escaped) = characters.next() {
                    current.push(escaped);
                }
                started_argument = true;
            }
            (None, '\\') => {
                current.push(character);
                started_argument = true;
            }
            (None, '$') => {
                if expand_variable(&mut characters, lookup, &mut current)? {
                    started_argument = true;
                }
            }
            (None, '~')
                if !started_argument
                    && characters
                        .peek()
                        .is_none_or(|next| next.is_whitespace() || matches!(next, '/' | '\\')) =>
            {
                match lookup("HOME").or_else(|| lookup("USERPROFILE")) {
                    Some(home) => current.push_str(&home),
                    None => current.push('~'),
                }
                started_argument = true;
            }
            (None, character) if character.is_whitespace() => {
                if started_argument {
                    args.push(finish_word(std::mem::take(&mut current), &mut wildcards));
                    started_argument = false;
                }
            }
            (None, character) => {
                if matches!(character, '*' | '?' | '[') {
                    wildcards.push(current.len());
                }
                current.push(character);
                started_argument = true;
            }
        }
    }

    if quote.is_some() {
        return Err("missing closing quote".to_string());
    }

    if started_argument {
        args.push(finish_word(current, &mut wildcards));
    }

    Ok(args)
}

/// Expand the variable after a `$` that was just read. Returns whether text was
/// added, so an empty unquoted expansion does not create an argument.
fn expand_variable(
    characters: &mut std::iter::Peekable<std::str::Chars<'_>>,
    lookup: Lookup,
    output: &mut String,
) -> Result<bool, String> {
    let name = match characters.peek().copied() {
        Some('{') => {
            characters.next();
            let mut name = String::new();
            loop {
                match characters.next() {
                    Some('}') => break,
                    Some(character) => name.push(character),
                    None => return Err("missing closing brace in variable expansion".to_string()),
                }
            }
            if !crate::utils::shell_env::is_valid_name(&name) && name != "?" {
                return Err(format!("invalid variable name: {name}"));
            }
            name
        }
        Some('?') => {
            characters.next();
            "?".to_string()
        }
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {
            let mut name = String::new();
            while let Some(&next) = characters.peek() {
                if !(next.is_ascii_alphanumeric() || next == '_') {
                    break;
                }
                name.push(next);
                characters.next();
            }
            name
        }
        _ => {
            output.push('$');
            return Ok(true);
        }
    };

    let value = lookup(&name).unwrap_or_default();
    output.push_str(&value);
    Ok(!value.is_empty())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Connector {
    /// Run unconditionally: the first command, or after `;`.
    Then,
    /// Run only if the previous command succeeded (`&&`).
    And,
    /// Run only if the previous command failed (`||`).
    Or,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Segment {
    pub connector: Connector,
    pub text: String,
    /// Ended by `&`: the pipeline runs as a background job.
    pub background: bool,
}

/// Split a line at the `;`, `&&` and `||` that are neither quoted nor escaped.
///
/// The text of each segment is returned untouched, so quotes and variables are
/// handled later, right before that segment runs. Pipes and redirections stay in
/// the text for [`split_stages`]; background jobs are rejected instead of being
/// passed to a program as arguments.
pub fn split_chain(input: &str) -> Result<Vec<Segment>, String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut connector = Connector::Then;
    let mut quote: Option<Quote> = None;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => {
                quote = None;
                current.push(character);
            }
            (Some(Quote::Double), '\\') if matches!(characters.peek(), Some('"' | '\\' | '$')) => {
                current.push(character);
                current.extend(characters.next());
            }
            // A substitution is opaque: its operators belong to its own line.
            (Some(Quote::Double) | None, '$') if characters.peek() == Some(&'(') => {
                current.push(character);
                current.extend(characters.next());
                let mut inner = String::new();
                scan_substitution(|| characters.next(), |c| inner.push(c))?;
                current.push_str(&inner);
            }
            (Some(_), character) => current.push(character),
            (None, '\'') => {
                quote = Some(Quote::Single);
                current.push(character);
            }
            (None, '"') => {
                quote = Some(Quote::Double);
                current.push(character);
            }
            (None, '\\') => {
                current.push(character);
                current.extend(characters.next());
            }
            (None, ';') => {
                finish_segment(&mut segments, &mut current, connector, ";")?;
                connector = Connector::Then;
            }
            (None, '&') if characters.peek() == Some(&'&') => {
                characters.next();
                finish_segment(&mut segments, &mut current, connector, "&&")?;
                connector = Connector::And;
            }
            (None, '|') if characters.peek() == Some(&'|') => {
                characters.next();
                finish_segment(&mut segments, &mut current, connector, "||")?;
                connector = Connector::Or;
            }
            // `>&` belongs to a redirection (`2>&1`), a lone `&` is a background job.
            (None, '&') if current.ends_with('>') => current.push(character),
            // A lone `&` ends a command and runs it in the background.
            (None, '&') => {
                if current.trim().is_empty() {
                    return Err("syntax error near unexpected token '&'".to_string());
                }
                if connector != Connector::Then {
                    return Err(
                        "a background job cannot follow && or ||: only a single command can run in the background"
                            .to_string(),
                    );
                }
                segments.push(Segment {
                    connector,
                    text: std::mem::take(&mut current),
                    background: true,
                });
                connector = Connector::Then;
            }
            // Pipes and redirections stay in the text: `split_stages` reads them.
            (None, character) => current.push(character),
        }
    }

    if quote.is_some() {
        return Err("missing closing quote".to_string());
    }
    if !current.trim().is_empty() {
        segments.push(Segment {
            connector,
            text: current,
            background: false,
        });
    } else if connector != Connector::Then {
        let operator = if connector == Connector::And {
            "&&"
        } else {
            "||"
        };
        return Err(format!("expected a command after {operator}"));
    }

    Ok(segments)
}

/// Read the rest of a `$(...)` whose `$(` was already consumed, up to and
/// including the matching `)`. Parentheses inside quotes do not count, and the
/// substitution has its own quoting: `"$(echo ")")"` is one substitution. `next`
/// supplies the characters, `keep` receives each one that was read.
pub(crate) fn scan_substitution(
    mut next: impl FnMut() -> Option<char>,
    mut keep: impl FnMut(char),
) -> Result<(), String> {
    let mut depth = 1_usize;
    let mut quote: Option<Quote> = None;
    while let Some(character) = next() {
        keep(character);
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => quote = None,
            (Some(Quote::Single), _) => {}
            (Some(Quote::Double), '\\') | (None, '\\') => {
                if let Some(escaped) = next() {
                    keep(escaped);
                }
            }
            (Some(Quote::Double), _) => {}
            (None, '\'') => quote = Some(Quote::Single),
            (None, '"') => quote = Some(Quote::Double),
            (None, '(') => depth += 1,
            (None, ')') => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            (None, _) => {}
        }
    }
    Err("missing closing parenthesis".to_string())
}

/// One redirection of a pipeline stage. Targets are raw text: quotes and
/// variables are expanded right before the stage runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Redirect {
    /// `< file`
    Stdin(String),
    /// `> file` or `>> file`
    Stdout { target: String, append: bool },
    /// `2> file` or `2>> file`
    Stderr { target: String, append: bool },
    /// `2>&1`
    StderrToStdout,
    /// `>&2` or `1>&2`
    StdoutToStderr,
}

/// One command of a pipeline: its words, without the redirections, and the
/// redirections themselves in the order they were written.
#[derive(Debug, Eq, PartialEq)]
pub struct Stage {
    pub text: String,
    pub redirects: Vec<Redirect>,
}

/// Split the text of a segment at the unquoted `|` and pull the redirections out
/// of each stage.
pub fn split_stages(input: &str) -> Result<Vec<Stage>, String> {
    let characters: Vec<char> = input.chars().collect();
    let mut position = 0;
    let mut stages = Vec::new();
    let mut text = String::new();
    let mut redirects = Vec::new();
    let mut quote: Option<Quote> = None;

    while position < characters.len() {
        let character = characters[position];
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => {
                quote = None;
                text.push(character);
                position += 1;
            }
            (Some(Quote::Double), '\\')
                if matches!(characters.get(position + 1), Some('"' | '\\' | '$')) =>
            {
                text.push(character);
                text.push(characters[position + 1]);
                position += 2;
            }
            (Some(Quote::Double) | None, '$') if characters.get(position + 1) == Some(&'(') => {
                text.push_str("$(");
                position += 2;
                let mut inner = String::new();
                scan_substitution(
                    || {
                        let next = characters.get(position).copied();
                        position += 1;
                        next
                    },
                    |c| inner.push(c),
                )?;
                text.push_str(&inner);
            }
            (Some(_), _) => {
                text.push(character);
                position += 1;
            }
            (None, '\'') => {
                quote = Some(Quote::Single);
                text.push(character);
                position += 1;
            }
            (None, '"') => {
                quote = Some(Quote::Double);
                text.push(character);
                position += 1;
            }
            (None, '\\') => {
                text.push(character);
                if let Some(escaped) = characters.get(position + 1) {
                    text.push(*escaped);
                }
                position += 2;
            }
            (None, '|') => {
                if text.trim().is_empty() {
                    return Err("syntax error near unexpected token '|'".to_string());
                }
                stages.push(Stage {
                    text: std::mem::take(&mut text),
                    redirects: std::mem::take(&mut redirects),
                });
                position += 1;
            }
            (None, '<') => {
                position += 1;
                redirects.push(Redirect::Stdin(read_target(
                    &characters,
                    &mut position,
                    "<",
                )?));
            }
            (None, '>') => {
                let descriptor = take_descriptor(&mut text);
                position += 1;
                let append = characters.get(position) == Some(&'>');
                if append {
                    position += 1;
                }
                if !append && characters.get(position) == Some(&'&') {
                    position += 1;
                    let destination = characters.get(position).copied();
                    position += 1;
                    match (descriptor, destination) {
                        (Some('2'), Some('1')) => redirects.push(Redirect::StderrToStdout),
                        (None | Some('1'), Some('2')) => redirects.push(Redirect::StdoutToStderr),
                        // `1>&1` and `2>&2` change nothing.
                        (Some('1'), Some('1')) | (Some('2'), Some('2')) => {}
                        _ => {
                            return Err("unsupported redirection: only 1 and 2 can be duplicated"
                                .to_string());
                        }
                    }
                    continue;
                }
                let operator = if append { ">>" } else { ">" };
                let target = read_target(&characters, &mut position, operator)?;
                redirects.push(if descriptor == Some('2') {
                    Redirect::Stderr { target, append }
                } else {
                    Redirect::Stdout { target, append }
                });
            }
            (None, _) => {
                text.push(character);
                position += 1;
            }
        }
    }

    if quote.is_some() {
        return Err("missing closing quote".to_string());
    }
    if text.trim().is_empty() {
        return Err(if stages.is_empty() {
            "expected a command before the redirection".to_string()
        } else {
            "expected a command after '|'".to_string()
        });
    }
    stages.push(Stage { text, redirects });
    Ok(stages)
}

/// A `1` or `2` just before `>` is the descriptor when it is a word of its own
/// (`cmd 2> log`), and an ordinary part of a word otherwise (`cmd file2> log`).
fn take_descriptor(text: &mut String) -> Option<char> {
    let mut characters = text.chars().rev();
    let last = characters.next()?;
    if !matches!(last, '1' | '2') {
        return None;
    }
    if characters
        .next()
        .is_some_and(|before| !before.is_whitespace())
    {
        return None;
    }
    text.pop();
    Some(last)
}

/// Read the word after a redirection operator, keeping its quotes and escapes.
fn read_target(
    characters: &[char],
    position: &mut usize,
    operator: &str,
) -> Result<String, String> {
    while characters.get(*position).is_some_and(|c| c.is_whitespace()) {
        *position += 1;
    }
    let mut target = String::new();
    let mut quote: Option<Quote> = None;
    while let Some(&character) = characters.get(*position) {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => quote = None,
            (Some(Quote::Double), '\\')
                if matches!(characters.get(*position + 1), Some('"' | '\\' | '$')) =>
            {
                target.push(character);
                *position += 1;
                target.push(characters[*position]);
                *position += 1;
                continue;
            }
            (Some(Quote::Double) | None, '$') if characters.get(*position + 1) == Some(&'(') => {
                target.push_str("$(");
                *position += 2;
                let mut index = *position;
                let mut inner = String::new();
                scan_substitution(
                    || {
                        let next = characters.get(index).copied();
                        index += 1;
                        next
                    },
                    |c| inner.push(c),
                )?;
                *position = index;
                target.push_str(&inner);
                continue;
            }
            (Some(_), _) => {}
            (None, '\'') => quote = Some(Quote::Single),
            (None, '"') => quote = Some(Quote::Double),
            (None, '\\') => {
                target.push(character);
                *position += 1;
                if let Some(&escaped) = characters.get(*position) {
                    target.push(escaped);
                    *position += 1;
                }
                continue;
            }
            (None, c) if c.is_whitespace() || matches!(c, '|' | '<' | '>') => break,
            (None, _) => {}
        }
        target.push(character);
        *position += 1;
    }
    if target.is_empty() {
        return Err(format!(
            "syntax error: expected a file name after '{operator}'"
        ));
    }
    Ok(target)
}

fn finish_segment(
    segments: &mut Vec<Segment>,
    current: &mut String,
    connector: Connector,
    operator: &str,
) -> Result<(), String> {
    if current.trim().is_empty() {
        return Err(format!("syntax error near unexpected token '{operator}'"));
    }
    segments.push(Segment {
        connector,
        text: std::mem::take(current),
        background: false,
    });
    Ok(())
}

pub fn continuation_prefix(line: &str) -> Option<&str> {
    let line = line.trim_end_matches(['\r', '\n']).trim_end();
    let trailing_ticks = line
        .chars()
        .rev()
        .take_while(|character| *character == '`')
        .count();
    (trailing_ticks % 2 == 1).then(|| {
        line.strip_suffix('`')
            .expect("a trailing tick was counted")
            .trim_end()
    })
}

pub fn normalize_multiline_input(input: &str) -> String {
    let mut lines = input.split('\n').peekable();
    let mut normalized = String::new();
    let mut first_line = true;
    while let Some(line) = lines.next() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let line = if lines.peek().is_some() {
            continuation_prefix(line).unwrap_or(line)
        } else {
            line
        };
        if !first_line {
            normalized.push(' ');
        }
        normalized.push_str(line);
        first_line = false;
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_input(input: &str) -> Result<Vec<String>, String> {
        parse_with(input, &|_| None)
    }

    fn chain(input: &str) -> Result<Vec<(Connector, String)>, String> {
        split_chain(input).map(|segments| {
            segments
                .into_iter()
                .map(|segment| (segment.connector, segment.text.trim().to_string()))
                .collect()
        })
    }

    fn step(connector: Connector, text: &str) -> (Connector, String) {
        (connector, text.to_string())
    }

    #[test]
    fn a_line_without_operator_is_one_segment() {
        assert_eq!(
            chain("echo hello"),
            Ok(vec![step(Connector::Then, "echo hello")])
        );
    }

    #[test]
    fn an_empty_line_has_no_segment() {
        assert_eq!(chain("   "), Ok(Vec::new()));
        assert_eq!(chain(""), Ok(Vec::new()));
    }

    #[test]
    fn the_three_operators_split_the_line() {
        assert_eq!(
            chain("a ; b && c || d"),
            Ok(vec![
                step(Connector::Then, "a"),
                step(Connector::Then, "b"),
                step(Connector::And, "c"),
                step(Connector::Or, "d"),
            ])
        );
    }

    #[test]
    fn operators_without_spaces_split_too() {
        assert_eq!(
            chain("a;b&&c||d"),
            Ok(vec![
                step(Connector::Then, "a"),
                step(Connector::Then, "b"),
                step(Connector::And, "c"),
                step(Connector::Or, "d"),
            ])
        );
    }

    #[test]
    fn quoted_operators_are_text() {
        assert_eq!(
            chain(r#"echo "a && b" ; echo 'c | d > e' "#),
            Ok(vec![
                step(Connector::Then, r#"echo "a && b""#),
                step(Connector::Then, "echo 'c | d > e'"),
            ])
        );
    }

    #[test]
    fn an_escaped_operator_is_text() {
        assert_eq!(
            chain(r"echo a\;b"),
            Ok(vec![step(Connector::Then, r"echo a\;b")])
        );
    }

    #[test]
    fn an_escaped_quote_does_not_end_a_double_quoted_string() {
        let line = r#"echo "say \" ; hi""#;

        assert_eq!(chain(line), Ok(vec![step(Connector::Then, line)]));
    }

    #[test]
    fn a_trailing_semicolon_is_allowed() {
        assert_eq!(chain("a ;"), Ok(vec![step(Connector::Then, "a")]));
    }

    #[test]
    fn a_missing_command_is_a_syntax_error() {
        assert_eq!(
            chain("&& a"),
            Err("syntax error near unexpected token '&&'".to_string())
        );
        assert_eq!(
            chain("a ;; b"),
            Err("syntax error near unexpected token ';'".to_string())
        );
        assert_eq!(
            chain("a &&"),
            Err("expected a command after &&".to_string())
        );
        assert_eq!(
            chain("a ||"),
            Err("expected a command after ||".to_string())
        );
    }

    #[test]
    fn pipes_and_redirections_stay_in_the_text_of_the_segment() {
        assert_eq!(chain("a | b"), Ok(vec![(Connector::Then, "a | b".into())]));
        assert_eq!(chain("a > f"), Ok(vec![(Connector::Then, "a > f".into())]));
        assert_eq!(chain("a < f"), Ok(vec![(Connector::Then, "a < f".into())]));
        assert_eq!(
            chain("a 2>&1 | b && c"),
            Ok(vec![
                (Connector::Then, "a 2>&1 | b".into()),
                (Connector::And, "c".into()),
            ])
        );
    }

    fn background(input: &str) -> Result<Vec<(String, bool)>, String> {
        split_chain(input).map(|segments| {
            segments
                .into_iter()
                .map(|segment| (segment.text.trim().to_string(), segment.background))
                .collect()
        })
    }

    #[test]
    fn a_lone_ampersand_runs_the_command_in_the_background() {
        assert_eq!(background("sleep 5 &"), Ok(vec![("sleep 5".into(), true)]));
        assert_eq!(
            background("build & test"),
            Ok(vec![("build".into(), true), ("test".into(), false)])
        );
        assert_eq!(background("a | b &"), Ok(vec![("a | b".into(), true)]));
        assert_eq!(
            background("a & b &"),
            Ok(vec![("a".into(), true), ("b".into(), true)])
        );
        assert_eq!(
            background("a & ; b"),
            Err("syntax error near unexpected token ';'".to_string())
        );
    }

    #[test]
    fn a_background_job_cannot_follow_an_and_or_list() {
        for line in ["a && b &", "a || b &"] {
            assert!(
                background(line).is_err_and(|error| error.contains("cannot follow")),
                "{line}"
            );
        }
    }

    #[test]
    fn a_misplaced_ampersand_is_a_syntax_error() {
        assert_eq!(
            background("& a"),
            Err("syntax error near unexpected token '&'".to_string())
        );
        assert_eq!(
            background("a && & b"),
            Err("syntax error near unexpected token '&'".to_string())
        );
    }

    #[test]
    fn an_ampersand_in_a_redirection_is_not_a_background_job() {
        assert_eq!(background("a 2>&1 &"), Ok(vec![("a 2>&1".into(), true)]));
        assert_eq!(background("a >&2"), Ok(vec![("a >&2".into(), false)]));
    }

    fn stages(input: &str) -> Result<Vec<(String, Vec<Redirect>)>, String> {
        split_stages(input).map(|stages| {
            stages
                .into_iter()
                .map(|stage| (stage.text, stage.redirects))
                .collect()
        })
    }

    fn write(target: &str) -> Redirect {
        Redirect::Stdout {
            target: target.into(),
            append: false,
        }
    }

    #[test]
    fn a_plain_command_is_one_stage_without_redirections() {
        assert_eq!(stages("ls -l"), Ok(vec![("ls -l".into(), vec![])]));
    }

    #[test]
    fn a_pipe_splits_the_stages() {
        assert_eq!(
            stages("ls | grep rs | wc -l"),
            Ok(vec![
                ("ls ".into(), vec![]),
                (" grep rs ".into(), vec![]),
                (" wc -l".into(), vec![]),
            ])
        );
    }

    #[test]
    fn every_redirection_form_is_read() {
        assert_eq!(
            stages("cmd < in > out"),
            Ok(vec![(
                "cmd  ".into(),
                vec![Redirect::Stdin("in".into()), write("out")]
            )])
        );
        assert_eq!(
            stages("cmd >> log"),
            Ok(vec![(
                "cmd ".into(),
                vec![Redirect::Stdout {
                    target: "log".into(),
                    append: true
                }]
            )])
        );
        assert_eq!(
            stages("cmd 2> err"),
            Ok(vec![(
                "cmd ".into(),
                vec![Redirect::Stderr {
                    target: "err".into(),
                    append: false
                }]
            )])
        );
        assert_eq!(
            stages("cmd 2>> err"),
            Ok(vec![(
                "cmd ".into(),
                vec![Redirect::Stderr {
                    target: "err".into(),
                    append: true
                }]
            )])
        );
        assert_eq!(
            stages("cmd 1> out"),
            Ok(vec![("cmd ".into(), vec![write("out")])])
        );
    }

    #[test]
    fn descriptors_can_be_duplicated() {
        assert_eq!(
            stages("cmd 2>&1"),
            Ok(vec![("cmd ".into(), vec![Redirect::StderrToStdout])])
        );
        assert_eq!(
            stages("cmd >&2"),
            Ok(vec![("cmd ".into(), vec![Redirect::StdoutToStderr])])
        );
        assert_eq!(
            stages("cmd 1>&2"),
            Ok(vec![("cmd ".into(), vec![Redirect::StdoutToStderr])])
        );
        assert_eq!(
            stages("cmd > out 2>&1"),
            Ok(vec![(
                "cmd  ".into(),
                vec![write("out"), Redirect::StderrToStdout]
            )])
        );
        assert_eq!(stages("cmd 1>&1"), Ok(vec![("cmd ".into(), vec![])]));
        assert!(stages("cmd 2>&3").is_err());
    }

    #[test]
    fn a_number_inside_a_word_is_not_a_descriptor() {
        assert_eq!(
            stages("echo file2> out"),
            Ok(vec![("echo file2".into(), vec![write("out")])])
        );
        // A `2` argument followed by a space is an argument.
        assert_eq!(
            stages("echo 2 > out"),
            Ok(vec![("echo 2 ".into(), vec![write("out")])])
        );
    }

    #[test]
    fn redirections_can_come_before_the_words_and_without_spaces() {
        assert_eq!(
            stages(">out echo hi"),
            Ok(vec![(" echo hi".into(), vec![write("out")])])
        );
        assert_eq!(
            stages("echo hi>out"),
            Ok(vec![("echo hi".into(), vec![write("out")])])
        );
        assert_eq!(
            stages("a|b"),
            Ok(vec![("a".into(), vec![]), ("b".into(), vec![])])
        );
    }

    #[test]
    fn quoted_operators_are_ordinary_text() {
        assert_eq!(
            stages("echo \"a | b > c\" 'd < e'"),
            Ok(vec![("echo \"a | b > c\" 'd < e'".into(), vec![])])
        );
        assert_eq!(
            stages("echo a\\|b \\> c"),
            Ok(vec![("echo a\\|b \\> c".into(), vec![])])
        );
    }

    #[test]
    fn a_target_keeps_its_quotes_for_the_later_expansion() {
        assert_eq!(
            stages("cmd > \"my file.txt\""),
            Ok(vec![("cmd ".into(), vec![write("\"my file.txt\"")])])
        );
        assert_eq!(
            stages("cmd > $HOME/out"),
            Ok(vec![("cmd ".into(), vec![write("$HOME/out")])])
        );
        assert_eq!(
            stages("cmd > a\\ b"),
            Ok(vec![("cmd ".into(), vec![write("a\\ b")])])
        );
    }

    #[test]
    fn syntax_errors_are_reported() {
        assert_eq!(
            stages("| b"),
            Err("syntax error near unexpected token '|'".to_string())
        );
        assert_eq!(
            stages("a | | b"),
            Err("syntax error near unexpected token '|'".to_string())
        );
        assert_eq!(
            stages("a |"),
            Err("expected a command after '|'".to_string())
        );
        assert_eq!(
            stages("a >"),
            Err("syntax error: expected a file name after '>'".to_string())
        );
        assert_eq!(
            stages("a >> | b"),
            Err("syntax error: expected a file name after '>>'".to_string())
        );
        assert_eq!(
            stages("a <"),
            Err("syntax error: expected a file name after '<'".to_string())
        );
        assert_eq!(
            stages("> out"),
            Err("expected a command before the redirection".to_string())
        );
        assert_eq!(stages("echo 'a"), Err("missing closing quote".to_string()));
    }

    #[test]
    fn an_unclosed_quote_is_reported() {
        assert_eq!(
            chain("echo 'a ; b"),
            Err("missing closing quote".to_string())
        );
    }

    fn variables(name: &str) -> Option<String> {
        match name {
            "NAME" => Some("world".to_string()),
            "EMPTY" => Some(String::new()),
            "HOME" => Some("/home/test".to_string()),
            "?" => Some("1".to_string()),
            "PATH_LIKE" => Some("a b".to_string()),
            _ => None,
        }
    }

    fn expanded(input: &str) -> Result<Vec<String>, String> {
        parse_with(input, &variables)
    }

    fn words(values: &[&str]) -> Result<Vec<String>, String> {
        Ok(values.iter().map(ToString::to_string).collect())
    }

    #[test]
    fn a_variable_is_expanded_outside_quotes() {
        assert_eq!(expanded("echo $NAME"), words(&["echo", "world"]));
        assert_eq!(
            expanded("echo pre$NAME-post"),
            words(&["echo", "preworld-post"])
        );
    }

    #[test]
    fn braces_end_the_variable_name() {
        assert_eq!(expanded("echo ${NAME}s"), words(&["echo", "worlds"]));
    }

    #[test]
    fn a_variable_is_expanded_inside_double_quotes() {
        assert_eq!(
            expanded(r#"echo "hello $NAME""#),
            words(&["echo", "hello world"])
        );
    }

    #[test]
    fn a_variable_is_not_expanded_inside_single_quotes() {
        assert_eq!(expanded("echo '$NAME'"), words(&["echo", "$NAME"]));
    }

    #[test]
    fn an_escaped_dollar_is_literal() {
        assert_eq!(expanded(r"echo \$NAME"), words(&["echo", "$NAME"]));
        assert_eq!(expanded(r#"echo "\$NAME""#), words(&["echo", "$NAME"]));
    }

    #[test]
    fn an_unset_variable_expands_to_nothing() {
        assert_eq!(expanded("echo $MISSING end"), words(&["echo", "end"]));
        assert_eq!(
            expanded("echo pre${MISSING}post"),
            words(&["echo", "prepost"])
        );
    }

    #[test]
    fn an_empty_unquoted_expansion_removes_its_argument() {
        assert_eq!(expanded("echo $EMPTY end"), words(&["echo", "end"]));
    }

    #[test]
    fn an_empty_quoted_expansion_keeps_an_empty_argument() {
        assert_eq!(
            expanded(r#"echo "$EMPTY" end"#),
            words(&["echo", "", "end"])
        );
    }

    #[test]
    fn an_expansion_is_never_split_into_several_arguments() {
        assert_eq!(expanded("echo $PATH_LIKE"), words(&["echo", "a b"]));
    }

    #[test]
    fn the_last_status_is_available() {
        assert_eq!(expanded("echo $?"), words(&["echo", "1"]));
        assert_eq!(expanded("echo ${?}"), words(&["echo", "1"]));
    }

    #[test]
    fn a_lone_dollar_stays_literal() {
        assert_eq!(expanded("echo $ cost$"), words(&["echo", "$", "cost$"]));
        assert_eq!(expanded("echo $5"), words(&["echo", "$5"]));
    }

    #[test]
    fn an_unclosed_brace_is_an_error() {
        assert_eq!(
            expanded("echo ${NAME"),
            Err("missing closing brace in variable expansion".to_string())
        );
    }

    #[test]
    fn an_invalid_name_in_braces_is_an_error() {
        assert_eq!(
            expanded("echo ${1a}"),
            Err("invalid variable name: 1a".to_string())
        );
        assert_eq!(
            expanded("echo ${}"),
            Err("invalid variable name: ".to_string())
        );
    }

    #[test]
    fn a_leading_tilde_becomes_the_home_directory() {
        assert_eq!(expanded("cd ~"), words(&["cd", "/home/test"]));
        assert_eq!(expanded("cd ~/code"), words(&["cd", "/home/test/code"]));
    }

    #[test]
    fn a_tilde_elsewhere_or_quoted_stays_literal() {
        assert_eq!(expanded("echo a~b"), words(&["echo", "a~b"]));
        assert_eq!(expanded("echo ~user"), words(&["echo", "~user"]));
        assert_eq!(expanded(r#"echo "~""#), words(&["echo", "~"]));
        assert_eq!(expanded("echo '~'"), words(&["echo", "~"]));
    }

    #[test]
    fn a_tilde_without_a_home_directory_stays_literal() {
        assert_eq!(parse_with("cd ~", &|_| None), words(&["cd", "~"]));
    }

    #[test]
    fn parse_input_splits_words() {
        assert_eq!(
            parse_input("ls -la src"),
            Ok(vec!["ls".into(), "-la".into(), "src".into()])
        );
    }

    #[test]
    fn parse_input_ignores_extra_spaces() {
        assert_eq!(
            parse_input("  cd   target  "),
            Ok(vec!["cd".into(), "target".into()])
        );
    }

    #[test]
    fn parse_input_keeps_quoted_paths_together() {
        assert_eq!(
            parse_input(r#"cd "my folder""#),
            Ok(vec!["cd".into(), "my folder".into()])
        );
    }

    #[test]
    fn parse_input_keeps_single_quoted_paths_together() {
        assert_eq!(
            parse_input("cat 'my folder/file.txt'"),
            Ok(vec!["cat".into(), "my folder/file.txt".into()])
        );
    }

    #[test]
    fn parse_input_keeps_empty_quoted_arguments() {
        assert_eq!(
            parse_input("command \"\""),
            Ok(vec!["command".into(), "".into()])
        );
        assert_eq!(
            parse_input("command ''"),
            Ok(vec!["command".into(), "".into()])
        );
    }

    #[test]
    fn parse_input_supports_escaped_spaces_and_quotes() {
        assert_eq!(
            parse_input(r#"cat my\ file\ with\ spaces.txt"#),
            Ok(vec!["cat".into(), "my file with spaces.txt".into()])
        );
        assert_eq!(
            parse_input(r#"cat "a\\b\"c.txt""#),
            Ok(vec!["cat".into(), "a\\b\"c.txt".into()])
        );
    }

    #[test]
    fn parse_input_rejects_unclosed_quotes() {
        assert_eq!(
            parse_input(r#"cd "my folder"#),
            Err("missing closing quote".to_string())
        );
    }

    #[test]
    fn parse_input_rejects_unclosed_single_quotes() {
        assert_eq!(
            parse_input("cat 'my folder"),
            Err("missing closing quote".to_string())
        );
    }

    #[test]
    fn continuation_prefix_requires_an_odd_trailing_backtick_count() {
        assert_eq!(
            continuation_prefix("echo hello `  \r\n"),
            Some("echo hello")
        );
        assert_eq!(continuation_prefix("echo hello ``"), None);
        assert_eq!(continuation_prefix("echo hello"), None);
    }

    #[test]
    fn normalize_multiline_input_joins_lines_and_removes_continuation_markers() {
        assert_eq!(
            normalize_multiline_input("echo hello `\nworld"),
            "echo hello world"
        );
        assert_eq!(
            normalize_multiline_input("echo hello\nworld"),
            "echo hello world"
        );
        assert_eq!(
            normalize_multiline_input("echo unfinished `"),
            "echo unfinished `"
        );
    }

    #[test]
    fn parse_input_preserves_windows_path_separators() {
        assert_eq!(
            parse_input(r#"cat "C:\Users\some folder\file.txt""#),
            Ok(vec!["cat".into(), r"C:\Users\some folder\file.txt".into()])
        );
    }

    #[test]
    fn parse_makes_path() {
        assert_eq!(
            parse_input(r#"mkdir "new folder""#),
            Ok(vec!["mkdir".into(), "new folder".into()])
        )
    }

    #[test]
    fn parse_make_file() {
        assert_eq!(
            parse_input(r#"touch "new_file.txt""#),
            Ok(vec!["touch".into(), "new_file.txt".into()])
        )
    }

    fn patterns(input: &str) -> Vec<Option<String>> {
        parse_words(input, &|name| (name == "X").then(|| "*.rs".to_string()))
            .expect("the line should parse")
            .into_iter()
            .map(|word| word.pattern)
            .collect()
    }

    #[test]
    fn an_unquoted_wildcard_makes_a_pattern() {
        assert_eq!(
            patterns("ls *.rs a?c [ab]x plain"),
            [
                None,
                Some("*.rs".into()),
                Some("a?c".into()),
                Some("[ab]x".into()),
                None
            ]
        );
    }

    #[test]
    fn quoted_wildcards_are_literal() {
        assert_eq!(patterns("echo '*.rs' \"a?c\""), [None, None, None]);
    }

    #[test]
    fn a_quoted_wildcard_inside_a_pattern_word_is_escaped() {
        assert_eq!(
            patterns("echo \"[draft]\"*.md"),
            [None, Some("\\[draft]*.md".into())]
        );
        assert_eq!(patterns("echo 'a*b'*"), [None, Some("a\\*b*".into())]);
    }

    #[cfg(not(windows))]
    #[test]
    fn a_backslash_protects_a_wildcard_and_stays_literal_elsewhere() {
        assert_eq!(patterns("echo \\*"), [None, None]);
        assert_eq!(
            parse_with("echo \\*", &|_| None),
            Ok(vec!["echo".to_string(), "*".to_string()])
        );
        assert_eq!(patterns("echo a\\*b*"), [None, Some("a\\*b*".into())]);
    }

    #[test]
    fn the_value_of_a_variable_is_not_a_pattern() {
        assert_eq!(patterns("echo $X"), [None, None]);
        // The variable's own `*` is literal even when the word has a real one.
        assert_eq!(patterns("echo $X*"), [None, Some("\\*.rs*".into())]);
    }

    #[test]
    fn parse_with_never_matches_files() {
        assert_eq!(
            parse_with("ls *.rs", &|_| None),
            Ok(vec!["ls".to_string(), "*.rs".to_string()])
        );
    }

    #[test]
    fn operators_inside_a_substitution_do_not_split_the_line() {
        assert_eq!(
            chain("echo $(a ; b && c | d) ; e"),
            Ok(vec![
                (Connector::Then, "echo $(a ; b && c | d)".into()),
                (Connector::Then, "e".into()),
            ])
        );
        assert_eq!(
            chain("echo $(a & b)"),
            Ok(vec![(Connector::Then, "echo $(a & b)".into())])
        );
    }

    #[test]
    fn quotes_and_parentheses_inside_a_substitution_are_balanced() {
        assert_eq!(
            chain("echo $(echo \")\" ; echo '(') ; x"),
            Ok(vec![
                (Connector::Then, "echo $(echo \")\" ; echo '(')".into()),
                (Connector::Then, "x".into()),
            ])
        );
        assert_eq!(
            chain("echo \"$(echo \")\")\" ; x"),
            Ok(vec![
                (Connector::Then, "echo \"$(echo \")\")\"".into()),
                (Connector::Then, "x".into()),
            ])
        );
        assert_eq!(chain("echo $(a $(b ; c) d) ; x").map(|s| s.len()), Ok(2));
    }

    #[test]
    fn a_substitution_that_is_never_closed_is_a_syntax_error() {
        assert_eq!(
            chain("echo $(a ; b"),
            Err("missing closing parenthesis".to_string())
        );
        assert_eq!(
            stages("echo $(a | b"),
            Err("missing closing parenthesis".to_string())
        );
    }

    #[test]
    fn a_literal_dollar_paren_does_not_open_a_substitution() {
        assert_eq!(chain("echo '$(a ; b' ; x").map(|s| s.len()), Ok(2));
        assert_eq!(chain("echo \\$(a ; b) ; x").map(|s| s.len()), Ok(3));
    }

    #[test]
    fn pipes_and_redirections_inside_a_substitution_stay_in_it() {
        assert_eq!(
            stages("echo $(a | b > c) | cat"),
            Ok(vec![
                ("echo $(a | b > c) ".into(), vec![]),
                (" cat".into(), vec![]),
            ])
        );
        assert_eq!(
            stages("cmd > $(echo a b) 2> err"),
            Ok(vec![(
                "cmd  ".into(),
                vec![
                    write("$(echo a b)"),
                    Redirect::Stderr {
                        target: "err".into(),
                        append: false
                    }
                ]
            )])
        );
    }
}
