#[derive(Clone, Copy, Eq, PartialEq)]
enum Quote {
    Single,
    Double,
}

/// Look up the value of a variable name (`?` is the last status).
pub type Lookup<'a> = &'a dyn Fn(&str) -> Option<String>;

pub fn parse_with(input: &str, lookup: Lookup) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
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
                    next.is_whitespace() || matches!(next, '\\' | '\'' | '"' | '$')
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
                    args.push(std::mem::take(&mut current));
                    started_argument = false;
                }
            }
            (None, character) => {
                current.push(character);
                started_argument = true;
            }
        }
    }

    if quote.is_some() {
        return Err("missing closing quote".to_string());
    }

    if started_argument {
        args.push(current);
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
}
