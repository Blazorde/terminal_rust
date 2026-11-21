//! The `test` and `[` commands: conditions on files, strings and integers
//! (see ADR-0016). They are built in so that `if [ -f file ]` works the same on
//! every system, Windows included, where there is no `test` program.

use std::fs;
use std::path::Path;

/// The outcome of a condition: true, false, or a mistake in the expression.
pub type Outcome = Result<bool, String>;

/// Evaluate `test ARGS` (`bracket` is false) or `[ ARGS ]` (`bracket` is true,
/// and `args` ends with the closing `]`).
pub fn evaluate(args: &[String], bracket: bool) -> Outcome {
    let name = if bracket { "[" } else { "test" };
    let mut words: Vec<&str> = args.iter().map(String::as_str).collect();
    if bracket {
        if words.last() != Some(&"]") {
            return Err("[: missing ']'".to_string());
        }
        words.pop();
    }
    if words.is_empty() {
        return Ok(false);
    }
    let mut parser = Parser {
        words: &words,
        position: 0,
        name,
    };
    let value = parser.or()?;
    match words.get(parser.position) {
        None => Ok(value),
        Some(extra) => Err(format!("{name}: unexpected argument '{extra}'")),
    }
}

struct Parser<'a> {
    words: &'a [&'a str],
    position: usize,
    name: &'a str,
}

const UNARY: [&str; 12] = [
    "-e", "-f", "-d", "-L", "-h", "-r", "-w", "-x", "-s", "-z", "-n", "-t",
];
const BINARY: [&str; 9] = ["=", "==", "!=", "-eq", "-ne", "-lt", "-le", "-gt", "-ge"];

impl<'a> Parser<'a> {
    fn peek(&self, offset: usize) -> Option<&'a str> {
        self.words.get(self.position + offset).copied()
    }

    fn or(&mut self) -> Outcome {
        let mut value = self.and()?;
        while self.peek(0) == Some("-o") {
            self.position += 1;
            let right = self.and()?;
            value = value || right;
        }
        Ok(value)
    }

    fn and(&mut self) -> Outcome {
        let mut value = self.not()?;
        while self.peek(0) == Some("-a") {
            self.position += 1;
            let right = self.not()?;
            value = value && right;
        }
        Ok(value)
    }

    fn not(&mut self) -> Outcome {
        // `! x` negates; a lone `!` is the string "!".
        if self.peek(0) == Some("!") && self.peek(1).is_some() {
            self.position += 1;
            return Ok(!self.not()?);
        }
        self.primary()
    }

    fn primary(&mut self) -> Outcome {
        let Some(first) = self.peek(0) else {
            return Err(format!("{}: argument expected", self.name));
        };
        // `a OP b` first: `-n = -n` compares two strings.
        if let (Some(operator), Some(right)) = (self.peek(1), self.peek(2))
            && BINARY.contains(&operator)
        {
            self.position += 3;
            return self.binary(first, operator, right);
        }
        if UNARY.contains(&first)
            && let Some(operand) = self.peek(1)
        {
            self.position += 2;
            return Ok(unary(first, operand));
        }
        self.position += 1;
        Ok(!first.is_empty())
    }

    fn binary(&self, left: &str, operator: &str, right: &str) -> Outcome {
        match operator {
            "=" | "==" => Ok(left == right),
            "!=" => Ok(left != right),
            _ => {
                let number = |text: &str| {
                    text.trim().parse::<i64>().map_err(|_| {
                        format!("{}: integer expression expected: '{text}'", self.name)
                    })
                };
                let (left, right) = (number(left)?, number(right)?);
                Ok(match operator {
                    "-eq" => left == right,
                    "-ne" => left != right,
                    "-lt" => left < right,
                    "-le" => left <= right,
                    "-gt" => left > right,
                    _ => left >= right,
                })
            }
        }
    }
}

fn unary(operator: &str, operand: &str) -> bool {
    let path = Path::new(operand);
    match operator {
        "-z" => operand.is_empty(),
        "-n" => !operand.is_empty(),
        "-e" => fs::metadata(path).is_ok(),
        "-f" => fs::metadata(path).is_ok_and(|metadata| metadata.is_file()),
        "-d" => fs::metadata(path).is_ok_and(|metadata| metadata.is_dir()),
        "-L" | "-h" => fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()),
        "-s" => fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0),
        "-r" => readable(path),
        "-w" => fs::metadata(path).is_ok_and(|metadata| !metadata.permissions().readonly()),
        "-x" => executable(path),
        // `-t FD` asks about a terminal; a shell built-in has none to offer.
        _ => false,
    }
}

fn readable(path: &Path) -> bool {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::read_dir(path).is_ok(),
        Ok(_) => fs::File::open(path).is_ok(),
        Err(_) => false,
    }
}

#[cfg(unix)]
fn executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &Path) -> bool {
    // No execute bit here: a file with a runnable extension, or a directory.
    fs::metadata(path).is_ok_and(|metadata| {
        metadata.is_dir()
            || path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    ["exe", "cmd", "bat", "com"]
                        .iter()
                        .any(|runnable| extension.eq_ignore_ascii_case(runnable))
                })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;

    fn test(args: &[&str]) -> Outcome {
        evaluate(
            &args.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            false,
        )
    }

    fn bracket(args: &[&str]) -> Outcome {
        evaluate(
            &args.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            true,
        )
    }

    #[test]
    fn no_argument_is_false_and_one_argument_is_a_non_empty_string() {
        assert_eq!(test(&[]), Ok(false));
        assert_eq!(test(&["x"]), Ok(true));
        assert_eq!(test(&[""]), Ok(false));
        assert_eq!(test(&["-f"]), Ok(true));
        assert_eq!(test(&["!"]), Ok(true));
    }

    #[test]
    fn strings_compare_and_test_for_emptiness() {
        assert_eq!(test(&["a", "=", "a"]), Ok(true));
        assert_eq!(test(&["a", "==", "b"]), Ok(false));
        assert_eq!(test(&["a", "!=", "b"]), Ok(true));
        assert_eq!(test(&["-z", ""]), Ok(true));
        assert_eq!(test(&["-z", "x"]), Ok(false));
        assert_eq!(test(&["-n", "x"]), Ok(true));
        assert_eq!(test(&["-n", "=", "-n"]), Ok(true));
    }

    #[test]
    fn integers_compare_by_value_not_by_text() {
        assert_eq!(test(&["10", "-gt", "9"]), Ok(true));
        assert_eq!(test(&["9", "-gt", "10"]), Ok(false));
        assert_eq!(test(&["3", "-eq", "03"]), Ok(true));
        assert_eq!(test(&["-2", "-lt", "1"]), Ok(true));
        assert_eq!(test(&["2", "-le", "2"]), Ok(true));
        assert_eq!(test(&["2", "-ge", "3"]), Ok(false));
        assert_eq!(test(&["2", "-ne", "3"]), Ok(true));
    }

    #[test]
    fn a_non_integer_is_an_error() {
        let error = test(&["x", "-eq", "1"]).unwrap_err();
        assert!(
            error.contains("integer expression expected: 'x'"),
            "{error}"
        );
    }

    #[test]
    fn negation_and_or_combine_conditions() {
        assert_eq!(test(&["!", "a", "=", "b"]), Ok(true));
        assert_eq!(test(&["!", "!", "x"]), Ok(true));
        assert_eq!(test(&["a", "=", "a", "-a", "b", "=", "b"]), Ok(true));
        assert_eq!(test(&["a", "=", "a", "-a", "b", "=", "c"]), Ok(false));
        assert_eq!(test(&["a", "=", "b", "-o", "c", "=", "c"]), Ok(true));
        // `-a` binds tighter than `-o`.
        assert_eq!(
            test(&["x", "=", "y", "-a", "x", "=", "x", "-o", "z", "=", "z"]),
            Ok(true)
        );
    }

    #[test]
    fn brackets_need_their_closing_bracket() {
        assert_eq!(bracket(&["a", "=", "a", "]"]), Ok(true));
        assert_eq!(bracket(&["]"]), Ok(false));
        assert_eq!(bracket(&["a", "=", "a"]), Err("[: missing ']'".to_string()));
    }

    #[test]
    fn extra_arguments_are_an_error() {
        let error = test(&["a", "b"]).unwrap_err();
        assert!(error.contains("unexpected argument 'b'"), "{error}");
    }

    #[test]
    fn file_tests_look_at_the_file_system() {
        let directory = temp_path("test", "files");
        fs::create_dir(&directory).expect("fixture");
        let file = directory.join("data.txt");
        let empty = directory.join("empty.txt");
        fs::write(&file, "x").expect("fixture");
        fs::write(&empty, "").expect("fixture");
        let (file, empty, dir, missing) = (
            file.to_str().expect("UTF-8"),
            empty.to_str().expect("UTF-8"),
            directory.to_str().expect("UTF-8"),
            "terminal-rust-no-such-file",
        );

        assert_eq!(test(&["-e", file]), Ok(true));
        assert_eq!(test(&["-e", missing]), Ok(false));
        assert_eq!(test(&["-f", file]), Ok(true));
        assert_eq!(test(&["-f", dir]), Ok(false));
        assert_eq!(test(&["-d", dir]), Ok(true));
        assert_eq!(test(&["-d", file]), Ok(false));
        assert_eq!(test(&["-s", file]), Ok(true));
        assert_eq!(test(&["-s", empty]), Ok(false));
        assert_eq!(test(&["-r", file]), Ok(true));
        assert_eq!(test(&["-w", file]), Ok(true));
        assert_eq!(test(&["!", "-e", missing]), Ok(true));
        assert_eq!(bracket(&["-f", file, "-a", "-d", dir, "]"]), Ok(true));
        fs::remove_dir_all(directory).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn execute_permission_and_symbolic_links_are_tested() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = temp_path("test", "unix");
        fs::create_dir(&directory).expect("fixture");
        let program = directory.join("run");
        let plain = directory.join("plain");
        let link = directory.join("link");
        fs::write(&program, "#!/bin/sh\n").expect("fixture");
        fs::write(&plain, "x").expect("fixture");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).expect("chmod");
        fs::set_permissions(&plain, fs::Permissions::from_mode(0o644)).expect("chmod");
        symlink(&plain, &link).expect("symlink");

        assert_eq!(test(&["-x", program.to_str().expect("UTF-8")]), Ok(true));
        assert_eq!(test(&["-x", plain.to_str().expect("UTF-8")]), Ok(false));
        assert_eq!(test(&["-L", link.to_str().expect("UTF-8")]), Ok(true));
        assert_eq!(test(&["-L", plain.to_str().expect("UTF-8")]), Ok(false));
        fs::remove_dir_all(directory).expect("cleanup");
    }
}
