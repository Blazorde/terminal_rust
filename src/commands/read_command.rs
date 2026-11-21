//! The `read` command: read one line and store it in variables.
//!
//! `read [-r] [-p PROMPT] [NAME...]` takes its line from a pipe or a redirection
//! when there is one, and from the terminal otherwise. The line is split at
//! blanks, one field per name, and the last name receives the rest of the line
//! (`read first rest`). With no name the line goes to `REPLY`. A backslash is
//! always literal, as with `read -r`.

use std::io::BufRead;

use crate::utils::{pipe_input, shell_env};

/// What `read` was asked to do.
#[derive(Debug, Eq, PartialEq)]
pub struct Request {
    pub prompt: Option<String>,
    pub names: Vec<String>,
}

/// Read the options and the variable names of `read`.
pub fn parse(args: &[String]) -> Result<Request, String> {
    let mut prompt = None;
    let mut names = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        match argument.as_str() {
            "-r" => {}
            "-p" => {
                index += 1;
                match args.get(index) {
                    Some(text) => prompt = Some(text.clone()),
                    None => return Err("read: -p needs a prompt".to_string()),
                }
            }
            "--" => {
                names.extend(args[index + 1..].iter().cloned());
                break;
            }
            option if option.starts_with('-') && option.len() > 1 => {
                return Err(format!("read: unsupported option '{option}'"));
            }
            _ => names.push(argument.clone()),
        }
        index += 1;
    }
    if names.is_empty() {
        names.push("REPLY".to_string());
    }
    if let Some(bad) = names.iter().find(|name| !shell_env::is_valid_name(name)) {
        return Err(format!("read: not a valid name: {bad}"));
    }
    Ok(Request { prompt, names })
}

/// Take the next line of input: from the pipe or redirection of this command, or
/// from `fallback` (the terminal) when there is none. `None` at the end of the input.
pub fn next_line(fallback: &mut dyn BufRead) -> Option<String> {
    match pipe_input::next_line() {
        pipe_input::Line::Text(line) => Some(line),
        pipe_input::Line::End => None,
        pipe_input::Line::NoInput => {
            let mut line = String::new();
            if fallback.read_line(&mut line).ok()? == 0 {
                return None;
            }
            Some(line.trim_end_matches(['\n', '\r']).to_string())
        }
    }
}

/// Split `line` into one value per name: fields are separated by blanks, and the
/// last name gets the rest of the line without its leading blanks.
pub fn split_fields(line: &str, names: &[String]) -> Vec<(String, String)> {
    let mut rest = line.trim_start();
    let mut values = Vec::with_capacity(names.len());
    for (position, name) in names.iter().enumerate() {
        if position + 1 == names.len() {
            values.push((name.clone(), rest.trim_end().to_string()));
            break;
        }
        match rest.split_once(char::is_whitespace) {
            Some((field, remainder)) => {
                values.push((name.clone(), field.to_string()));
                rest = remainder.trim_start();
            }
            None => {
                values.push((name.clone(), rest.to_string()));
                rest = "";
            }
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn pairs(values: &[(&str, &str)]) -> Vec<(String, String)> {
        values
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn without_a_name_the_line_goes_to_reply() {
        let request = parse(&[]).expect("valid");
        assert_eq!(request.names, names(&["REPLY"]));
        assert_eq!(request.prompt, None);
    }

    #[test]
    fn options_and_names_are_read() {
        let request = parse(&names(&["-r", "-p", "Name: ", "first", "rest"])).expect("valid");
        assert_eq!(request.prompt.as_deref(), Some("Name: "));
        assert_eq!(request.names, names(&["first", "rest"]));
        assert_eq!(
            parse(&names(&["--", "-x"])).unwrap_err(),
            "read: not a valid name: -x"
        );
    }

    #[test]
    fn mistakes_are_reported() {
        assert_eq!(
            parse(&names(&["-p"])).unwrap_err(),
            "read: -p needs a prompt"
        );
        assert_eq!(
            parse(&names(&["-s"])).unwrap_err(),
            "read: unsupported option '-s'"
        );
        assert_eq!(
            parse(&names(&["1bad"])).unwrap_err(),
            "read: not a valid name: 1bad"
        );
    }

    #[test]
    fn one_name_takes_the_whole_line_trimmed() {
        assert_eq!(
            split_fields("  hello   big world  ", &names(&["line"])),
            pairs(&[("line", "hello   big world")])
        );
    }

    #[test]
    fn fields_go_to_names_and_the_last_takes_the_rest() {
        assert_eq!(
            split_fields("a b c d", &names(&["x", "y"])),
            pairs(&[("x", "a"), ("y", "b c d")])
        );
        assert_eq!(
            split_fields("a   b", &names(&["x", "y", "z"])),
            pairs(&[("x", "a"), ("y", "b"), ("z", "")])
        );
        assert_eq!(
            split_fields("", &names(&["x", "y"])),
            pairs(&[("x", ""), ("y", "")])
        );
    }

    #[test]
    fn a_backslash_is_kept() {
        assert_eq!(
            split_fields(r"a\b c", &names(&["x", "y"])),
            pairs(&[("x", r"a\b"), ("y", "c")])
        );
    }

    #[test]
    fn the_lines_come_from_the_pipe_one_by_one_and_from_the_terminal_without_one() {
        pipe_input::provide(Some(b"from pipe\nsecond\n".to_vec()));
        let mut terminal = Cursor::new(b"from terminal\n".to_vec());
        assert_eq!(next_line(&mut terminal).as_deref(), Some("from pipe"));
        // The pipe still has a line: the terminal is not read yet.
        assert_eq!(next_line(&mut terminal).as_deref(), Some("second"));
        // Then the input is over, and a finished input is not the terminal.
        assert_eq!(next_line(&mut terminal), None);
        pipe_input::provide(None);
        assert_eq!(next_line(&mut terminal).as_deref(), Some("from terminal"));
        assert_eq!(next_line(&mut terminal), None);
    }

    #[test]
    fn an_empty_pipe_is_the_end_of_input_and_a_crlf_is_removed() {
        pipe_input::provide(Some(Vec::new()));
        let mut terminal = Cursor::new(Vec::new());
        assert_eq!(next_line(&mut terminal), None);
        pipe_input::provide(Some(b"windows\r\n".to_vec()));
        assert_eq!(next_line(&mut terminal).as_deref(), Some("windows"));
    }

    #[test]
    fn a_pipe_without_a_final_newline_still_gives_its_line() {
        pipe_input::provide(Some(b"no newline".to_vec()));
        let mut terminal = Cursor::new(Vec::new());
        assert_eq!(next_line(&mut terminal).as_deref(), Some("no newline"));
    }
}
