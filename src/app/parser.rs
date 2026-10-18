#[derive(Clone, Copy, Eq, PartialEq)]
enum Quote {
    Single,
    Double,
}

pub fn parse_input(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quote: Option<Quote> = None;
    let mut started_argument = false;
    let mut characters = input.chars().peekable();

    while let Some(character) = characters.next() {
        match (quote, character) {
            (Some(Quote::Single), '\'') | (Some(Quote::Double), '"') => quote = None,
            (Some(Quote::Single), character) => current.push(character),
            (Some(Quote::Double), '\\') if matches!(characters.peek(), Some('"' | '\\')) => {
                current.push(characters.next().expect("peeked character should exist"));
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
                    next.is_whitespace() || matches!(next, '\\' | '\'' | '"')
                }) =>
            {
                current.push(characters.next().expect("peeked character should exist"));
                started_argument = true;
            }
            (None, '\\') => {
                current.push(character);
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

#[cfg(test)]
mod tests {
    use super::*;

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
