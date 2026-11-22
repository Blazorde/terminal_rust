#[derive(Clone, Copy, Eq, PartialEq)]
enum Quote {
    Single,
    Double,
}

/// Look up the value of a variable name (`?` is the last status, `1` to `9` the
/// positional parameters, `#` their number, `*` all of them joined by spaces, and
/// `@` all of them joined by [`PARAMETER_SEPARATOR`]).
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

/// Between the positional parameters in the value of `@`: `$@` and `"$@"` become
/// one word per parameter, so the separator must not be a character that can be
/// typed in a command line.
pub const PARAMETER_SEPARATOR: char = '\u{1f}';

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

/// Whether the expansion that starts after a `$` gives one word per piece: `$@`
/// and `${=NAME}`.
fn splits_into_words(characters: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut ahead = characters.clone();
    match ahead.next() {
        Some('@') => true,
        Some('{') => ahead.next() == Some('=') || is_all_elements(characters),
        _ => false,
    }
}

/// Whether what follows a `$` is `{name[@]}` or `{!name[@]}`: every element of an array,
/// one word each.
fn is_all_elements(characters: &std::iter::Peekable<std::str::Chars<'_>>) -> bool {
    let mut ahead = characters.clone();
    if ahead.next() != Some('{') {
        return false;
    }
    let content: String = ahead.take_while(|character| *character != '}').collect();
    // `name[@]`, `!name[@]`, and their slices `name[@]:1:2` and `@:2`.
    let head = content.split(':').next().unwrap_or_default();
    head == "@"
        || (head.ends_with("[@]")
            && crate::utils::shell_env::is_valid_name(
                head.trim_start_matches('!').trim_end_matches("[@]"),
            ))
}

/// Turn the separators of an expanded `$@` or `${=NAME}` into word breaks: the text before the
/// first one joins the word being written, and each later parameter starts a word.
fn split_parameters(current: &mut String, args: &mut Vec<Word>, wildcards: &mut Vec<usize>) {
    if !current.contains(PARAMETER_SEPARATOR) {
        return;
    }
    let all = std::mem::take(current);
    let mut parts = all.split(PARAMETER_SEPARATOR);
    *current = parts.next().unwrap_or_default().to_string();
    for part in parts {
        args.push(finish_word(std::mem::take(current), wildcards));
        *current = part.to_string();
    }
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
    // Whether a word had begun before the quote that is open, and whether a
    // `"$@"` with no parameter must leave no word at all.
    let mut started_before_quote = false;
    let mut drop_empty_quotes = false;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => {
                quote = None;
                if drop_empty_quotes && current.is_empty() {
                    started_argument = false;
                }
                drop_empty_quotes = false;
            }
            (Some(Quote::Single), character) => current.push(character),
            (Some(Quote::Double), '\\') if matches!(characters.peek(), Some('"' | '\\' | '$')) => {
                if let Some(escaped) = characters.next() {
                    current.push(escaped);
                }
            }
            (Some(Quote::Double), '$') => {
                let splits = splits_into_words(&characters);
                let parameters = characters.peek() == Some(&'@') || is_all_elements(&characters);
                let added = expand_variable(&mut characters, lookup, &mut current)?;
                if splits {
                    split_parameters(&mut current, &mut args, &mut wildcards);
                }
                if parameters {
                    // `"$@"` with no parameter is no word, not an empty one.
                    drop_empty_quotes = !added && current.is_empty() && !started_before_quote;
                }
            }
            (Some(Quote::Double), character) => current.push(character),
            (None, '\'') => {
                quote = Some(Quote::Single);
                started_before_quote = started_argument;
                started_argument = true;
            }
            (None, '"') => {
                quote = Some(Quote::Double);
                started_before_quote = started_argument;
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
                let splits = splits_into_words(&characters);
                if expand_variable(&mut characters, lookup, &mut current)? {
                    started_argument = true;
                }
                if splits {
                    split_parameters(&mut current, &mut args, &mut wildcards);
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
            let mut content = String::new();
            let mut depth = 1_usize;
            loop {
                match characters.next() {
                    Some('{') => {
                        depth += 1;
                        content.push('{');
                    }
                    Some('}') => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        content.push('}');
                    }
                    Some(character) => content.push(character),
                    None => return Err("missing closing brace in variable expansion".to_string()),
                }
            }
            return expand_braced(&content, lookup, output);
        }
        Some(special @ ('?' | '#' | '@' | '*' | '$' | '!')) => {
            characters.next();
            special.to_string()
        }
        Some(digit) if digit.is_ascii_digit() => {
            characters.next();
            digit.to_string()
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

/// Expand the inside of `${...}`: `NAME`, `#NAME` (length), and the forms with a
/// word, `NAME:-w` and `NAME-w` (default), `NAME:+w` and `NAME+w` (alternative),
/// `NAME:?w` and `NAME?w` (error). With a colon, an empty value counts as unset.
fn expand_braced(content: &str, lookup: Lookup, output: &mut String) -> Result<bool, String> {
    // `${=NAME}`: split the value at blanks into separate words, as `$@` does. This is
    // the explicit form of what other shells do to every unquoted variable (ADR-0009).
    if let Some(rest) = content.strip_prefix('=') {
        let mut value = String::new();
        expand_braced(rest, lookup, &mut value)?;
        let words = value
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(&PARAMETER_SEPARATOR.to_string());
        output.push_str(&words);
        return Ok(!words.is_empty());
    }
    let valid = |name: &str| {
        crate::utils::shell_env::is_valid_name(name)
            || matches!(name, "?" | "#" | "@" | "*" | "$" | "!")
            || (!name.is_empty() && name.chars().all(|character| character.is_ascii_digit()))
            || is_array_reference(name)
    };
    // `${#name[@]}` is the number of elements, `${!name[@]}` the list of indices.
    if let Some(name) = content.strip_prefix('#')
        && valid(name)
    {
        let all = is_array_reference(name) && (name.ends_with("[@]") || name.ends_with("[*]"));
        let counted = if all {
            lookup(&format!("#{name}")).unwrap_or_default()
        } else {
            lookup(name).unwrap_or_default().chars().count().to_string()
        };
        output.push_str(&counted);
        return Ok(true);
    }
    if let Some(name) = content.strip_prefix('!')
        && name.ends_with("[@]")
        && is_array_reference(name)
    {
        let indices = lookup(&format!("!{name}")).unwrap_or_default();
        output.push_str(&indices);
        return Ok(!indices.is_empty());
    }
    let name_end = if content.starts_with(['?', '#', '@', '*', '$', '!']) {
        1
    } else {
        let identifier = content
            .find(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .unwrap_or(content.len());
        // `name[index]`: the brackets belong to the name.
        match content[identifier..]
            .strip_prefix('[')
            .and_then(|rest| rest.find(']'))
        {
            Some(close) => identifier + 1 + close + 1,
            None => identifier,
        }
    };
    let (name, operation) = content.split_at(name_end);
    if !valid(name) {
        return Err(format!("invalid variable name: {name}"));
    }
    let value = lookup(name);
    // The operators that work on the value: `#`, `%`, `/`, `:offset`, `^`, `,`.
    let expand = |text: &str| expand_word(text, lookup);
    if (name.ends_with("[@]") || name == "@")
        && let Some(slice) = operation.strip_prefix(':')
        && super::parameter::is_substring(slice)
    {
        // A slice of an array: elements, not characters.
        let separator = PARAMETER_SEPARATOR.to_string();
        let value = value.unwrap_or_default();
        let mut elements: Vec<String> = if value.is_empty() {
            Vec::new()
        } else {
            value
                .split(PARAMETER_SEPARATOR)
                .map(str::to_string)
                .collect()
        };
        // `${@:2}` counts from `$1` as 1: `$0` is the element before the first parameter.
        if name == "@" {
            elements.insert(0, lookup("0").unwrap_or_default());
        }
        let (start, end) = super::parameter::range(elements.len(), slice, &expand)?;
        let text = elements[start..end].join(&separator);
        output.push_str(&text);
        return Ok(!text.is_empty());
    }
    if let Some(text) =
        super::parameter::apply(operation, value.as_deref().unwrap_or_default(), &expand)?
    {
        output.push_str(&text);
        return Ok(!text.is_empty());
    }
    let (colon, operation) = match operation.strip_prefix(':') {
        Some(rest) => (true, rest),
        None => (false, operation),
    };
    // With a colon an empty value is treated as unset.
    let set = value
        .as_ref()
        .is_some_and(|value| !colon || !value.is_empty());
    let text = match operation.chars().next() {
        None if colon => return Err(format!("bad substitution: ${{{content}}}")),
        None => value.unwrap_or_default(),
        Some('-') if set => value.unwrap_or_default(),
        Some('-') => expand_word(&operation[1..], lookup)?,
        Some('+') if set => expand_word(&operation[1..], lookup)?,
        Some('+') => String::new(),
        Some('?') if set => value.unwrap_or_default(),
        Some('?') => {
            let message = expand_word(&operation[1..], lookup)?;
            return Err(if message.is_empty() {
                format!("{name}: parameter null or not set")
            } else {
                format!("{name}: {message}")
            });
        }
        Some('=') => {
            return Err(format!(
                "${{{content}}}: assigning in an expansion is not supported, use {name}=value"
            ));
        }
        Some(_) => return Err(format!("bad substitution: ${{{content}}}")),
    };
    output.push_str(&text);
    Ok(!text.is_empty())
}

/// `name[index]`, `name[@]` or `name[*]`: a reference to an element or to all of an array.
pub fn is_array_reference(name: &str) -> bool {
    let Some((identifier, rest)) = name.split_once('[') else {
        return false;
    };
    let Some(index) = rest.strip_suffix(']') else {
        return false;
    };
    crate::utils::shell_env::is_valid_name(identifier) && !index.is_empty() && !index.contains(']')
}

/// Expand the word of a `${NAME:-word}`: variables, quotes and escapes, but no
/// splitting and no patterns.
fn expand_word(word: &str, lookup: Lookup) -> Result<String, String> {
    let mut output = String::new();
    let mut characters = word.chars().peekable();
    let mut quote: Option<Quote> = None;
    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => quote = None,
            (Some(Quote::Single), character) => output.push(character),
            (None, '\'') => quote = Some(Quote::Single),
            (None, '"') => quote = Some(Quote::Double),
            (_, '$') => {
                expand_variable(&mut characters, lookup, &mut output)?;
            }
            (_, '\\') => {
                if let Some(escaped) = characters.next() {
                    output.push(escaped);
                }
            }
            (_, character) => output.push(character),
        }
    }
    Ok(output)
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
    // Inside `[[ … ]]`, `&&`, `||`, `<`, `>`, `(` and `)` are operators of the test.
    let mut in_test = false;
    // How many `( … )` subshells are open: a `(` that starts a command opens one.
    let mut subshells = 0_usize;
    // The `(` inside a command of a subshell (`x=(1 2)`), whose `)` is not the subshell's.
    let mut literal_parens = 0_usize;

    while let Some(character) = characters.next() {
        if in_test && quote.is_none() && closes_double_bracket(&current, character) {
            in_test = false;
        }
        if !in_test && quote.is_none() && character.is_whitespace() && current.trim() == "[[" {
            in_test = true;
        }
        match (quote, character) {
            (None, '&' | '|' | ';') if in_test && !closes_double_bracket(&current, character) => {
                current.push(character);
            }
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
            // `( list )`: the `(` is a word of its own, and the `)` that closes it starts a
            // new command, so the flow parser reads them like `{` and `}`. The `()` of
            // `name()` and a `)` with no subshell open (a `case` pattern) are plain text.
            (None, '(') if starts_command(&current) && !in_test => {
                current.push_str("( ");
                subshells += 1;
            }
            (None, '(') if subshells > 0 => {
                current.push(character);
                literal_parens += 1;
            }
            (None, ')') if literal_parens > 0 => {
                current.push(character);
                literal_parens -= 1;
            }
            (None, ')') if subshells > 0 && !current.ends_with('(') => {
                finish_segment(&mut segments, &mut current, connector, ")")?;
                connector = Connector::Then;
                current.push(')');
                subshells -= 1;
            }
            // `;;` ends an arm of a `case`: it is a segment of its own.
            (None, ';') if characters.peek() == Some(&';') => {
                characters.next();
                if !current.trim().is_empty() {
                    finish_segment(&mut segments, &mut current, connector, ";;")?;
                } else if connector != Connector::Then {
                    return Err("syntax error near unexpected token ';;'".to_string());
                }
                segments.push(Segment {
                    connector: Connector::Then,
                    text: ";;".to_string(),
                    background: false,
                });
                connector = Connector::Then;
            }
            (None, ';') => {
                finish_segment(&mut segments, &mut current, connector, ";")?;
                connector = Connector::Then;
            }
            // A newline ends a command like `;`, and is ignored when there is none
            // (a blank line, or a line break after `&&`, `||` or `&`).
            (None, '\n') => {
                if !current.trim().is_empty() {
                    finish_segment(&mut segments, &mut current, connector, "newline")?;
                    connector = Connector::Then;
                }
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

/// Whether what has been read of a command is empty or only keywords that are followed
/// by a command (`if`, `then`, `do`, `{`, …): a `(` here opens a subshell.
fn starts_command(current: &str) -> bool {
    let mut words = current.split_whitespace().peekable();
    if words.peek().is_none() {
        return true;
    }
    current.ends_with(char::is_whitespace)
        && words.all(|word| {
            // `name() {` before the `(`: the `()` of a function header is a word too.
            word.ends_with("()")
                || matches!(
                    word,
                    "if" | "then" | "elif" | "else" | "do" | "while" | "until" | "{" | "("
                )
        })
}

/// Whether `text` is a `[[ … ]]` command (it starts with the word `[[`).
fn opens_double_bracket(text: &str) -> bool {
    text.trim_start()
        .strip_prefix("[[")
        .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

/// Whether the text read so far ends with the closing `]]` and `next` ends that word.
fn closes_double_bracket(current: &str, next: char) -> bool {
    current.ends_with(" ]]") && (next.is_whitespace() || matches!(next, '&' | '|' | ';'))
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
    /// `<<< word`: the word and a newline are the input. A here-document is
    /// rewritten into one of these.
    HereString(String),
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
    // `[[ a < b || c ]]` is one command: its `<`, `>`, `|` and `&` are operators of its own.
    if opens_double_bracket(input) {
        return Ok(vec![Stage {
            text: input.to_string(),
            redirects: Vec::new(),
        }]);
    }
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
            (None, '<') if characters.get(position + 1) == Some(&'<') => {
                if characters.get(position + 2) != Some(&'<') {
                    return Err(
                        "syntax error: a here-document needs its body on the following lines"
                            .to_string(),
                    );
                }
                position += 3;
                redirects.push(Redirect::HereString(read_target(
                    &characters,
                    &mut position,
                    "<<<",
                )?));
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
    // A line that ends with the closing backquote of a substitution (`echo `date``)
    // is complete: the tick continues the line only when the ticks of the whole line
    // are odd in number, the others being pairs.
    let all_ticks = line.chars().filter(|character| *character == '`').count();
    (trailing_ticks % 2 == 1 && all_ticks % 2 == 1).then(|| {
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

    #[test]
    fn array_references_are_names_and_all_elements_split_into_words() {
        for name in ["a[0]", "a[@]", "a[*]", "a[$i]", "_x1[12]"] {
            assert!(is_array_reference(name), "{name}");
        }
        for name in ["a", "a[]", "a[1", "1a[0]", "a[1]]", "[0]", "a b[0]"] {
            assert!(!is_array_reference(name), "{name}");
        }
        let lookup = |name: &str| match name {
            "a[@]" => Some(format!("one{PARAMETER_SEPARATOR}two words")),
            "a[1]" => Some("two words".to_string()),
            "#a[@]" => Some("2".to_string()),
            "!a[@]" => Some(format!("0{PARAMETER_SEPARATOR}1")),
            _ => None,
        };
        let words = |text: &str| parse_args(text, &lookup).expect("parses");
        // Like `$@`, an element is never split again, quoted or not.
        assert_eq!(words("${a[@]}"), vec!["one", "two words"]);
        assert_eq!(words("\"${a[@]}\""), vec!["one", "two words"]);
        assert_eq!(words("\"${a[1]}\" x"), vec!["two words", "x"]);
        assert_eq!(words("${#a[@]}"), vec!["2"]);
        assert_eq!(words("${!a[@]}"), vec!["0", "1"]);
        // A missing array gives no word when it is all of them, and nothing otherwise.
        assert!(words("${b[@]}").is_empty());
        assert!(words("\"${b[@]}\"").is_empty());
    }

    #[test]
    fn a_paren_that_starts_a_command_opens_a_subshell() {
        let texts = |line: &str| {
            split_chain(line)
                .expect("splits")
                .into_iter()
                .map(|segment| segment.text.trim().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            texts("(cd d && ls) || echo no"),
            vec!["( cd d", "ls", ")", "echo no"]
        );
        assert_eq!(texts("(a; b)"), vec!["( a", "b", ")"]);
        assert_eq!(
            texts("if (a); then b; fi"),
            vec!["if ( a", ")", "then b", "fi"]
        );
        assert_eq!(texts("(a) > f"), vec!["( a", ") > f"]);
        // Not a subshell: the `()` of a function header, a `(` inside a command, a `)`
        // with nothing open (a `case` pattern), and the parentheses of `$(...)`.
        assert_eq!(texts("f() { echo; }"), vec!["f() { echo", "}"]);
        assert_eq!(
            texts("(f() { echo; }; f)"),
            vec!["( f() { echo", "}", "f", ")"]
        );
        assert_eq!(texts("echo a (b) c"), vec!["echo a (b) c"]);
        assert_eq!(
            texts("case x in a) echo; esac"),
            vec!["case x in a) echo", "esac"]
        );
        assert_eq!(texts("echo $(a; b)"), vec!["echo $(a; b)"]);
        assert_eq!(texts("echo \"(a)\""), vec!["echo \"(a)\""]);
    }

    #[test]
    fn a_double_bracket_keeps_its_operators_and_ends_at_the_closing_brackets() {
        let texts = |line: &str| {
            split_chain(line)
                .expect("splits")
                .into_iter()
                .map(|segment| segment.text.trim().to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(texts("[[ -f a && -f b ]]"), vec!["[[ -f a && -f b ]]"]);
        assert_eq!(
            texts("[[ a || b ]] && echo x"),
            vec!["[[ a || b ]]", "echo x"]
        );
        assert_eq!(texts("[[ a ]]&& echo x"), vec!["[[ a ]]", "echo x"]);
        assert_eq!(texts("[[ a ]]; echo x"), vec!["[[ a ]]", "echo x"]);
        // Only a command that starts with the word `[[` is a test.
        assert_eq!(texts("echo [[ a && b"), vec!["echo [[ a", "b"]);
        assert_eq!(texts("echo [[a && b"), vec!["echo [[a", "b"]);
        // A quoted word that looks like the closing brackets does not end it.
        assert_eq!(
            texts("[[ a == \"]]\" && b ]]"),
            vec!["[[ a == \"]]\" && b ]]"]
        );

        let stages = split_stages("[[ a < b | c > d ]]").expect("one stage");
        assert_eq!(stages.len(), 1);
        assert!(stages[0].redirects.is_empty());
    }

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
        // `;;` is the end of an arm of a `case`: it is a segment of its own here,
        // and the block parser rejects it outside a `case`.
        assert_eq!(
            chain("a ;; b"),
            Ok(vec![
                (Connector::Then, "a".into()),
                (Connector::Then, ";;".into()),
                (Connector::Then, "b".into())
            ])
        );
        assert_eq!(
            chain("a && ;; b"),
            Err("syntax error near unexpected token ';;'".to_string())
        );
        assert_eq!(chain(";;"), Ok(vec![(Connector::Then, ";;".into())]));
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
            "ACCENT" => Some("été".to_string()),
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
    fn a_default_replaces_an_unset_or_empty_variable() {
        assert_eq!(expanded("echo ${MISSING:-d}"), words(&["echo", "d"]));
        assert_eq!(expanded("echo ${EMPTY:-d}"), words(&["echo", "d"]));
        assert_eq!(expanded("echo ${NAME:-d}"), words(&["echo", "world"]));
        // Without a colon only an unset variable takes the default.
        assert_eq!(expanded("echo ${MISSING-d}"), words(&["echo", "d"]));
        assert_eq!(expanded("echo ${EMPTY-d} end"), words(&["echo", "end"]));
        assert_eq!(expanded("echo ${NAME-d}"), words(&["echo", "world"]));
    }

    #[test]
    fn the_word_of_an_expansion_has_variables_quotes_and_nesting() {
        assert_eq!(
            expanded("echo ${MISSING:-$NAME}"),
            words(&["echo", "world"])
        );
        assert_eq!(
            expanded("echo ${MISSING:-\"two words\"}"),
            words(&["echo", "two words"])
        );
        assert_eq!(
            expanded("echo ${MISSING:-${ALSO_MISSING:-deep}}"),
            words(&["echo", "deep"])
        );
        assert_eq!(
            expanded("echo ${MISSING:-'$NAME'}"),
            words(&["echo", "$NAME"])
        );
        assert_eq!(expanded("echo ${MISSING:-a}b"), words(&["echo", "ab"]));
    }

    #[test]
    fn an_alternative_applies_only_to_a_set_variable() {
        assert_eq!(expanded("echo ${NAME:+yes}"), words(&["echo", "yes"]));
        assert_eq!(expanded("echo ${EMPTY:+yes} end"), words(&["echo", "end"]));
        assert_eq!(expanded("echo ${EMPTY+yes}"), words(&["echo", "yes"]));
        assert_eq!(
            expanded("echo ${MISSING:+yes} end"),
            words(&["echo", "end"])
        );
    }

    #[test]
    fn a_length_counts_characters() {
        assert_eq!(expanded("echo ${#NAME}"), words(&["echo", "5"]));
        assert_eq!(expanded("echo ${#MISSING}"), words(&["echo", "0"]));
        assert_eq!(expanded("echo ${#PATH_LIKE}"), words(&["echo", "3"]));
        // Characters, not bytes: "été" is 5 bytes.
        assert_eq!(expanded("echo ${#ACCENT}"), words(&["echo", "3"]));
    }

    #[test]
    fn a_required_variable_reports_its_message() {
        assert_eq!(expanded("echo ${NAME:?needed}"), words(&["echo", "world"]));
        assert_eq!(
            expanded("echo ${MISSING:?needed}"),
            Err("MISSING: needed".to_string())
        );
        assert_eq!(
            expanded("echo ${EMPTY:?}"),
            Err("EMPTY: parameter null or not set".to_string())
        );
    }

    #[test]
    fn unsupported_forms_are_explained() {
        let assign = expanded("echo ${MISSING:=x}").unwrap_err();
        assert!(
            assign.contains("assigning in an expansion is not supported"),
            "{assign}"
        );
        let bad = expanded("echo ${NAME@Q}").unwrap_err();
        assert!(bad.contains("bad substitution"), "{bad}");
        assert!(
            expanded("echo ${NAME:}")
                .unwrap_err()
                .contains("bad substitution")
        );
    }

    #[test]
    fn the_default_is_literal_text_not_a_pattern_or_a_split() {
        let words = parse_words("echo ${MISSING:-*.txt}", &variables).expect("parses");
        assert_eq!(words[1].text, "*.txt");
        assert_eq!(words[1].pattern, None);
        assert_eq!(
            expanded("echo ${MISSING:-a b}"),
            super::tests::words(&["echo", "a b"])
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
        // `$5` is the fifth positional parameter, which is not set here.
        assert_eq!(expanded("echo $5"), words(&["echo"]));
    }

    fn with_parameters(name: &str) -> Option<String> {
        match name {
            "1" => Some("one".to_string()),
            "2" => Some("two words".to_string()),
            "#" => Some("2".to_string()),
            "*" => Some("one two words".to_string()),
            "@" => Some(format!("one{PARAMETER_SEPARATOR}two words")),
            _ => variables(name),
        }
    }

    fn without_parameters(name: &str) -> Option<String> {
        match name {
            "#" => Some("0".to_string()),
            "@" | "*" => Some(String::new()),
            _ => variables(name),
        }
    }

    fn with(input: &str) -> Result<Vec<String>, String> {
        parse_with(input, &with_parameters)
    }

    #[test]
    fn positional_parameters_have_a_number_and_a_count() {
        assert_eq!(with("echo $1 $2"), words(&["echo", "one", "two words"]));
        assert_eq!(with("echo ${1}x $#"), words(&["echo", "onex", "2"]));
        assert_eq!(with("echo ${#}"), words(&["echo", "2"]));
        assert_eq!(with("echo ${#1} ${#2}"), words(&["echo", "3", "9"]));
        assert_eq!(
            with("echo ${3:-fallback} ${1:-fallback}"),
            words(&["echo", "fallback", "one"])
        );
        // A single digit: `$10` is the first parameter and a zero.
        assert_eq!(with("echo $10"), words(&["echo", "one0"]));
        // An unset parameter gives nothing, like any unset variable.
        assert_eq!(with("echo a$3b"), words(&["echo", "ab"]));
        assert_eq!(with("echo ${12}"), words(&["echo"]));
        assert_eq!(
            expanded("echo ${1a}").unwrap_err(),
            "invalid variable name: 1a"
        );
    }

    #[test]
    fn at_gives_one_word_per_parameter_and_star_joins_them() {
        assert_eq!(with("echo $@"), words(&["echo", "one", "two words"]));
        assert_eq!(with("echo \"$@\""), words(&["echo", "one", "two words"]));
        assert_eq!(
            with("echo x\"$@\"y"),
            words(&["echo", "xone", "two wordsy"])
        );
        assert_eq!(with("echo $*"), words(&["echo", "one two words"]));
        assert_eq!(with("echo \"$*\""), words(&["echo", "one two words"]));
        assert_eq!(
            with("echo \"$@\" \"$*\""),
            words(&["echo", "one", "two words", "one two words"])
        );
        // A quoted `$@` next to other text keeps it in the same words.
        assert_eq!(
            with("echo \"[$@]\""),
            words(&["echo", "[one", "two words]"])
        );
    }

    #[test]
    fn an_equals_sign_splits_the_value_into_words_on_request() {
        let list = |name: &str| match name {
            "LIST" => Some("a  b\tc".to_string()),
            "ONE" => Some("single".to_string()),
            "PAD" => Some("  x  ".to_string()),
            "EMPTY" => Some(String::new()),
            "SPACED" => Some("two words".to_string()),
            _ => None,
        };
        let split = |input: &str| parse_with(input, &list);

        assert_eq!(split("echo ${=LIST}"), words(&["echo", "a", "b", "c"]));
        // Without it nothing is split, quoted or not.
        assert_eq!(split("echo $LIST"), words(&["echo", "a  b\tc"]));
        assert_eq!(split("echo \"$LIST\""), words(&["echo", "a  b\tc"]));
        // Joined to other text, the first and last pieces stay attached to it.
        assert_eq!(split("echo x${=LIST}y"), words(&["echo", "xa", "b", "cy"]));
        assert_eq!(split("echo ${=ONE}"), words(&["echo", "single"]));
        assert_eq!(split("echo ${=PAD}"), words(&["echo", "x"]));
        // Nothing to split gives no word, an unset name too.
        assert_eq!(split("echo ${=EMPTY} end"), words(&["echo", "end"]));
        assert_eq!(split("echo ${=MISSING} end"), words(&["echo", "end"]));
        // It also works on the result of the other forms and in double quotes.
        assert_eq!(split("echo ${=MISSING:-p q}"), words(&["echo", "p", "q"]));
        assert_eq!(split("echo ${=SPACED}"), words(&["echo", "two", "words"]));
        assert_eq!(split("echo \"${=LIST}\""), words(&["echo", "a", "b", "c"]));
        // A pattern character in a value is never a pattern.
        let star = |name: &str| (name == "G").then(|| "*.rs *.md".to_string());
        let parsed = parse_words("echo ${=G}", &star).expect("parses");
        assert_eq!(parsed.len(), 3);
        assert!(parsed.iter().all(|word| word.pattern.is_none()));
    }

    #[test]
    fn at_with_no_parameter_leaves_no_word() {
        let none = |input: &str| parse_with(input, &without_parameters);
        assert_eq!(none("echo \"$@\""), words(&["echo"]));
        assert_eq!(none("echo $@ end"), words(&["echo", "end"]));
        assert_eq!(none("echo a\"$@\"b"), words(&["echo", "ab"]));
        assert_eq!(none("echo \"\" \"$@\""), words(&["echo", ""]));
        assert_eq!(none("echo x\"$@\""), words(&["echo", "x"]));
        // Other text in the quotes still makes a word, even an empty-looking one.
        assert_eq!(none("echo \" $@\""), words(&["echo", " "]));
        assert_eq!(none("echo $# \"$*\""), words(&["echo", "0", ""]));
    }

    #[test]
    fn a_parameter_is_never_a_pattern_or_a_separator_in_text() {
        let lookup = |name: &str| match name {
            "1" => Some("*.rs".to_string()),
            _ => None,
        };
        let words = parse_words("echo $1", &lookup).expect("parses");
        assert_eq!(words[1].pattern, None);
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
        // The closing backquote of a substitution does not continue the line.
        assert_eq!(continuation_prefix("echo today=`date`"), None);
        assert_eq!(continuation_prefix("echo `date` `"), Some("echo `date`"));
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

    #[test]
    fn a_newline_ends_a_command_like_a_semicolon() {
        assert_eq!(
            chain("echo a\necho b\n\n  \necho c"),
            Ok(vec![
                (Connector::Then, "echo a".into()),
                (Connector::Then, "echo b".into()),
                (Connector::Then, "echo c".into()),
            ])
        );
        assert_eq!(
            chain("a &&\n b ||\n c"),
            Ok(vec![
                (Connector::Then, "a".into()),
                (Connector::And, "b".into()),
                (Connector::Or, "c".into()),
            ])
        );
        // Inside quotes or a substitution a newline is ordinary text.
        assert_eq!(
            chain("echo 'a\nb' \"c\nd\" $(x\ny)").map(|s| s.len()),
            Ok(1)
        );
    }
}
