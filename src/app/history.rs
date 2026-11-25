//! History expansion: `!!`, `!$`, `!^`, `!N`, `!-N` and `!prefix` (see ADR-0018).
//!
//! It applies to what is typed at the prompt, before the line is run and
//! recorded, so that `sudo !!` runs the previous command with `sudo` in front.
//! It is a plain text rewrite: the result goes through the normal parser.

/// Rewrite the history references of `line`. `None` when it has none. An event
/// that does not exist is an error, and the line is not to be run.
///
/// A `!` is left alone inside quotes, after a backslash or a `$`, at the end of
/// the line, before a blank, `=` or `(` (so `!=` and `[ ! -f x ]` still work).
pub fn expand(line: &str, history: &[String]) -> Result<Option<String>, String> {
    let characters: Vec<char> = line.chars().collect();
    let mut output = String::with_capacity(line.len());
    let mut changed = false;
    let mut quote: Option<char> = None;
    let mut position = 0;

    while position < characters.len() {
        let character = characters[position];
        match (quote, character) {
            (Some(open), close) if open == close => {
                quote = None;
                output.push(character);
            }
            // `\"` does not close double quotes.
            (Some('"'), '\\') => {
                output.push(character);
                if let Some(&escaped) = characters.get(position + 1) {
                    output.push(escaped);
                    position += 1;
                }
            }
            (Some(_), _) => output.push(character),
            (None, '\'' | '"') => {
                quote = Some(character);
                output.push(character);
            }
            (None, '\\') => {
                output.push(character);
                if let Some(&escaped) = characters.get(position + 1) {
                    output.push(escaped);
                    position += 1;
                }
            }
            (None, '!') if position > 0 && characters[position - 1] == '$' => {
                output.push(character);
            }
            (None, '!') => {
                let (replacement, consumed) = event(&characters[position + 1..], history)?;
                match replacement {
                    Some(text) => {
                        output.push_str(&text);
                        changed = true;
                        position += consumed;
                    }
                    None => output.push(character),
                }
            }
            (None, _) => output.push(character),
        }
        position += 1;
    }
    Ok(changed.then_some(output))
}

/// The text a reference stands for, and how many characters after the `!` it
/// used. `None` when the `!` is not a reference.
fn event(after: &[char], history: &[String]) -> Result<(Option<String>, usize), String> {
    let Some(&first) = after.first() else {
        return Ok((None, 0));
    };
    if first.is_whitespace() || matches!(first, '=' | '(') {
        return Ok((None, 0));
    }
    match first {
        '!' => Ok((Some(last_entry(history, "!!")?.to_string()), 1)),
        '$' => {
            let entry = last_entry(history, "!$")?;
            let word = words(entry).last().copied().unwrap_or_default();
            Ok((Some(word.to_string()), 1))
        }
        '^' => {
            let entry = last_entry(history, "!^")?;
            match words(entry).get(1) {
                Some(word) => Ok((Some((*word).to_string()), 1)),
                None => Err("!^: bad word specifier".to_string()),
            }
        }
        '-' | '0'..='9' => {
            let negative = first == '-';
            let digits: String = after[usize::from(negative)..]
                .iter()
                .take_while(|character| character.is_ascii_digit())
                .collect();
            if digits.is_empty() {
                // `!-x` is a search for a command that starts with `-x`.
                return search(after, history);
            }
            let number: usize = digits
                .parse()
                .map_err(|_| format!("!{digits}: event not found"))?;
            let consumed = usize::from(negative) + digits.len();
            let label = format!("!{}{digits}", if negative { "-" } else { "" });
            let index = if negative {
                history.len().checked_sub(number)
            } else {
                number.checked_sub(1)
            };
            match index.and_then(|index| history.get(index)) {
                Some(entry) if number > 0 => Ok((Some(entry.clone()), consumed)),
                _ => Err(format!("{label}: event not found")),
            }
        }
        _ => search(after, history),
    }
}

/// `!prefix`: the most recent command that starts with `prefix`.
fn search(after: &[char], history: &[String]) -> Result<(Option<String>, usize), String> {
    let prefix: String = after
        .iter()
        .take_while(|character| {
            !character.is_whitespace()
                && !matches!(
                    character,
                    ';' | '&' | '|' | '<' | '>' | '(' | ')' | '"' | '\''
                )
        })
        .collect();
    if prefix.is_empty() {
        return Ok((None, 0));
    }
    let consumed = prefix.chars().count();
    match history
        .iter()
        .rev()
        .find(|entry| entry.starts_with(&prefix))
    {
        Some(entry) => Ok((Some(entry.clone()), consumed)),
        None => Err(format!("!{prefix}: event not found")),
    }
}

fn last_entry<'a>(history: &'a [String], label: &str) -> Result<&'a str, String> {
    history
        .last()
        .map(String::as_str)
        .ok_or_else(|| format!("{label}: event not found"))
}

/// The words of a command as typed: split at blanks outside quotes, with the
/// quotes and backslashes kept so the word can be typed again as it was.
fn words(line: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match (quote, character) {
            (_, '\\') if quote != Some('\'') => {
                start.get_or_insert(index);
                escaped = true;
            }
            (Some(open), close) if open == close => quote = None,
            (Some(_), _) => {}
            (None, '\'' | '"') => {
                start.get_or_insert(index);
                quote = Some(character);
            }
            (None, blank) if blank.is_whitespace() => {
                if let Some(begin) = start.take() {
                    words.push(&line[begin..index]);
                }
            }
            (None, _) => {
                start.get_or_insert(index);
            }
        }
    }
    if let Some(begin) = start {
        words.push(&line[begin..]);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(entries: &[&str]) -> Vec<String> {
        entries.iter().map(ToString::to_string).collect()
    }

    fn run(line: &str, entries: &[&str]) -> Result<Option<String>, String> {
        expand(line, &history(entries))
    }

    fn some(text: &str) -> Result<Option<String>, String> {
        Ok(Some(text.to_string()))
    }

    #[test]
    fn a_line_without_a_reference_is_left_alone() {
        assert_eq!(run("echo hello", &["ls"]), Ok(None));
        assert_eq!(run("", &[]), Ok(None));
    }

    #[test]
    fn double_bang_is_the_previous_command() {
        assert_eq!(run("!!", &["ls -l", "pwd"]), some("pwd"));
        assert_eq!(run("sudo !!", &["apt update"]), some("sudo apt update"));
        assert_eq!(run("echo !! !!", &["x"]), some("echo x x"));
    }

    #[test]
    fn bang_dollar_is_the_last_word_and_bang_caret_the_first_argument() {
        assert_eq!(run("cd !$", &["mkdir -p a/b"]), some("cd a/b"));
        assert_eq!(run("echo !^", &["cp one two"]), some("echo one"));
        // With no argument the last word is the command itself.
        assert_eq!(run("echo !$", &["pwd"]), some("echo pwd"));
        assert_eq!(
            run("echo !^", &["pwd"]),
            Err("!^: bad word specifier".to_string())
        );
    }

    #[test]
    fn the_last_word_keeps_its_quotes() {
        assert_eq!(
            run("cat !$", &["echo hi > \"my file\""]),
            some("cat \"my file\"")
        );
        assert_eq!(
            run("cat !$", &["echo hi > my\\ file"]),
            some("cat my\\ file")
        );
        assert_eq!(run("cat !$", &["echo 'a b' 'c d'"]), some("cat 'c d'"));
    }

    #[test]
    fn a_number_counts_from_the_start_and_a_negative_one_from_the_end() {
        let entries = ["one", "two", "three"];
        assert_eq!(run("!1", &entries), some("one"));
        assert_eq!(run("!3", &entries), some("three"));
        assert_eq!(run("!-1", &entries), some("three"));
        assert_eq!(run("!-3", &entries), some("one"));
        assert_eq!(run("echo !2 end", &entries), some("echo two end"));
    }

    #[test]
    fn a_missing_event_is_an_error() {
        let entries = ["one", "two"];
        assert_eq!(run("!0", &entries), Err("!0: event not found".to_string()));
        assert_eq!(run("!3", &entries), Err("!3: event not found".to_string()));
        assert_eq!(
            run("!-3", &entries),
            Err("!-3: event not found".to_string())
        );
        assert_eq!(run("!!", &[]), Err("!!: event not found".to_string()));
        assert_eq!(run("!$", &[]), Err("!$: event not found".to_string()));
        assert_eq!(
            run("!nothing", &entries),
            Err("!nothing: event not found".to_string())
        );
        assert_eq!(
            run("!99999999999999999999999", &entries),
            Err("!99999999999999999999999: event not found".to_string())
        );
    }

    #[test]
    fn a_prefix_finds_the_most_recent_command_that_starts_with_it() {
        let entries = ["git status", "ls", "git log", "echo git"];
        assert_eq!(run("!git", &entries), some("git log"));
        assert_eq!(run("!ls", &entries), some("ls"));
        assert_eq!(run("!git -p", &entries), some("git log -p"));
        assert_eq!(run("!echo;pwd", &entries), some("echo git;pwd"));
    }

    #[test]
    fn a_dash_that_is_not_a_number_searches_for_a_command() {
        assert_eq!(run("!-x", &["-x run"]), some("-x run"));
        assert_eq!(run("!-x", &["ls"]), Err("!-x: event not found".to_string()));
    }

    #[test]
    fn quotes_a_backslash_and_a_dollar_protect_the_bang() {
        let entries = ["ls"];
        assert_eq!(run("echo '!!'", &entries), Ok(None));
        assert_eq!(run("echo \"!!\"", &entries), Ok(None));
        assert_eq!(run("echo \\!!", &entries), Ok(None));
        assert_eq!(run("echo $!", &entries), Ok(None));
        // `$!` is the pid of the last background job: text glued to it is not an event.
        assert_eq!(run("echo $!ls", &entries), Ok(None));
        // After a closed quote the next one is active again.
        assert_eq!(run("echo '!!' !!", &entries), some("echo '!!' ls"));
    }

    #[test]
    fn an_escaped_quote_does_not_close_double_quotes() {
        let entries = ["ls"];
        assert_eq!(run(r#"cat "a\"!!""#, &entries), Ok(None));
        assert_eq!(run(r#"cat "a\" !! b""#, &entries), Ok(None));
        // An escaped backslash leaves the quote after it free to close.
        assert_eq!(run(r#"cat "a\\" !!"#, &entries), some(r#"cat "a\\" ls"#));
        assert_eq!(run(r#"cat "a\"#, &entries), Ok(None));
    }

    #[test]
    fn a_bang_that_is_not_a_reference_stays() {
        let entries = ["ls"];
        assert_eq!(run("echo hi!", &entries), Ok(None));
        assert_eq!(run("echo !", &entries), Ok(None));
        assert_eq!(run("[ ! -f x ]", &entries), Ok(None));
        assert_eq!(run("[ a != b ]", &entries), Ok(None));
        assert_eq!(run("echo !(x)", &entries), Ok(None));
        assert_eq!(run("echo ! !!", &entries), some("echo ! ls"));
    }

    #[test]
    fn an_expansion_result_is_not_expanded_again() {
        assert_eq!(run("!!", &["echo !!"]), some("echo !!"));
    }

    #[test]
    fn words_split_outside_quotes_and_keep_them() {
        assert_eq!(words("a  b\tc"), vec!["a", "b", "c"]);
        assert_eq!(
            words("echo 'a b' \"c d\" e\\ f"),
            vec!["echo", "'a b'", "\"c d\"", "e\\ f"]
        );
        assert_eq!(words(""), Vec::<&str>::new());
        assert_eq!(words("x=\"a b\"c"), vec!["x=\"a b\"c"]);
    }

    /// A backslash inside single quotes is literal, so it must not hide the
    /// closing quote.
    #[test]
    fn a_backslash_in_single_quotes_does_not_escape() {
        assert_eq!(words("echo 'a\\' b"), vec!["echo", "'a\\'", "b"]);
    }

    #[test]
    fn random_text_never_panics() {
        const PIECES: [&str; 22] = [
            "!", "!!", "$", "^", "-", "0", "1", "99", "a", "git", " ", "\t", "'", "\"", "\\", ";",
            "|", "(", ")", "=", "é", "\n",
        ];
        let entries = history(&["one two", "git log", "", "x 'a b' \"c", "é"]);
        let mut seed = 0x2545_F491_4F6C_DD1D_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..100_000 {
            let length = 1 + (next() % 10) as usize;
            let text: String = (0..length)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect();
            let outcome = std::panic::catch_unwind(|| {
                let _ = expand(&text, &entries);
                let _ = words(&text);
            });
            assert!(outcome.is_ok(), "history expansion panicked on {text:?}");
        }
    }
}
