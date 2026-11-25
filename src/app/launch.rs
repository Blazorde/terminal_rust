//! The command line of the shell binary: which options were given and what to run.
//! Nothing here touches the system, the caller says which operands are files or commands.

use std::ffi::OsString;
use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Invocation {
    pub(crate) login: bool,
    /// `-i` was given; a terminal on standard input makes a shell interactive too.
    pub(crate) interactive: bool,
    pub(crate) action: Action,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Action {
    /// No operand: commands come from the terminal or from standard input.
    Shell,
    /// `-c STRING [NAME [ARGUMENT...]]`.
    CommandString {
        command: String,
        name: Option<String>,
        args: Vec<String>,
    },
    /// A script or one command, which [`target`] tells apart once the search path is
    /// known: a login shell changes it.
    Operands(Vec<String>),
    /// The operands are one command and its arguments.
    OneLine(Vec<String>),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Target {
    /// `FILE [ARGUMENT...]`.
    Script { path: String, args: Vec<String> },
    /// One command and its arguments.
    Command(Vec<String>),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum UsageError {
    MissingCommandString,
    NotUnicode { position: usize },
}

impl fmt::Display for UsageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCommandString => formatter.write_str("-c: option requires an argument"),
            Self::NotUnicode { position } => {
                write!(formatter, "argument {position} is not valid UTF-8 text")
            }
        }
    }
}

/// The arguments as text; the position in the error counts from 1.
pub(crate) fn decode(args: impl Iterator<Item = OsString>) -> Result<Vec<String>, UsageError> {
    args.enumerate()
        .map(|(index, argument)| {
            argument.into_string().map_err(|_| UsageError::NotUnicode {
                position: index + 1,
            })
        })
        .collect()
}

/// Read `args` (without the program name). Options come first: `-c`, `-i`, `-l`, alone
/// or grouped, `--login`, and `--` which ends them. Anything else starts the operands.
pub(crate) fn parse(argv0: &str, args: &[String]) -> Result<Invocation, UsageError> {
    // `login` and `su -` start a shell with a dash in front of its name.
    let mut login = argv0.starts_with('-');
    let mut interactive = false;
    let mut command_string = false;
    let mut options_ended = false;
    let mut operands = args;
    while let Some((argument, tail)) = operands.split_first() {
        match argument.as_str() {
            "--" => {
                operands = tail;
                options_ended = true;
                break;
            }
            "--login" => login = true,
            other => {
                let Some(letters) = short_options(other) else {
                    break;
                };
                command_string |= letters.contains('c');
                interactive |= letters.contains('i');
                login |= letters.contains('l');
            }
        }
        operands = tail;
    }

    let action = if command_string {
        let (command, tail) = operands
            .split_first()
            .ok_or(UsageError::MissingCommandString)?;
        Action::CommandString {
            command: command.clone(),
            name: tail.first().cloned(),
            args: tail.get(1..).unwrap_or_default().to_vec(),
        }
    } else {
        match operands.first() {
            None => Action::Shell,
            Some(first) if !options_ended && first.starts_with('-') => {
                Action::OneLine(operands.to_vec())
            }
            Some(_) => Action::Operands(operands.to_vec()),
        }
    };

    Ok(Invocation {
        login,
        interactive,
        action,
    })
}

/// What the operands ask for. A bare name that is a command stays that command, so that a
/// file of the current directory (a cloned repository, say) cannot take its place.
pub(crate) fn target(
    operands: Vec<String>,
    is_command: impl Fn(&str) -> bool,
    is_file: impl Fn(&str) -> bool,
) -> Target {
    match operands.split_first() {
        Some((first, tail))
            if (has_separator(first) || !is_command(first))
                && (is_file(first) || looks_like_script_path(first)) =>
        {
            Target::Script {
                path: first.clone(),
                args: tail.to_vec(),
            }
        }
        _ => Target::Command(operands),
    }
}

/// The letters of `-lc` and the like, when all of them are options of the shell.
fn short_options(argument: &str) -> Option<&str> {
    argument.strip_prefix('-').filter(|letters| {
        !letters.is_empty() && letters.chars().all(|letter| "cil".contains(letter))
    })
}

/// An operand that can only be meant as a file, so that a missing one is reported as such
/// and not as an unknown command.
fn looks_like_script_path(operand: &str) -> bool {
    has_separator(operand) || operand.ends_with(".sh") || operand.ends_with(".trs")
}

fn has_separator(operand: &str) -> bool {
    operand.contains('/') || (cfg!(windows) && operand.contains('\\'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn read(argv0: &str, args: &[&str]) -> Result<Invocation, UsageError> {
        parse(argv0, &strings(args))
    }

    fn action(args: &[&str]) -> Action {
        read("terminal_rust", args)
            .expect("the arguments are valid")
            .action
    }

    /// What `args` run when `ls` and `both` are commands, and when `existing`, `both`
    /// and the paths to `both` are files.
    fn run_of(args: &[&str]) -> Target {
        let Action::Operands(operands) = action(args) else {
            panic!("{args:?} should leave operands");
        };
        target(
            operands,
            |name| name.ends_with("ls") || name.ends_with("both"),
            |path| ["existing", "both", "./both", r".\both", "/bin/ls"].contains(&path),
        )
    }

    fn operands(values: &[&str]) -> Action {
        Action::Operands(strings(values))
    }

    fn one_command(values: &[&str]) -> Target {
        Target::Command(strings(values))
    }

    fn flags(argv0: &str, args: &[&str]) -> (bool, bool) {
        let invocation = read(argv0, args).expect("the arguments are valid");
        (invocation.login, invocation.interactive)
    }

    fn command_string(command: &str, name: Option<&str>, args: &[&str]) -> Action {
        Action::CommandString {
            command: command.into(),
            name: name.map(str::to_string),
            args: strings(args),
        }
    }

    fn script(path: &str, args: &[&str]) -> Target {
        Target::Script {
            path: path.into(),
            args: strings(args),
        }
    }

    #[test]
    fn no_operand_is_a_shell() {
        for args in [&[][..], &["-i"], &["-l"], &["--"], &["--login", "--"]] {
            assert_eq!(action(args), Action::Shell, "{args:?}");
        }
    }

    #[test]
    fn options_set_the_flags_alone_or_grouped() {
        assert_eq!(flags("terminal_rust", &[]), (false, false));
        assert_eq!(flags("terminal_rust", &["-l"]), (true, false));
        assert_eq!(flags("terminal_rust", &["--login"]), (true, false));
        assert_eq!(flags("terminal_rust", &["-i"]), (false, true));
        assert_eq!(flags("terminal_rust", &["-il"]), (true, true));
        assert_eq!(flags("terminal_rust", &["-i", "-l"]), (true, true));
        assert_eq!(flags("terminal_rust", &["-lc", "x"]), (true, false));
        assert_eq!(flags("terminal_rust", &["-ic", "x"]), (false, true));
    }

    #[test]
    fn a_dash_before_the_program_name_means_login() {
        assert_eq!(flags("-terminal_rust", &[]), (true, false));
        assert_eq!(flags("/usr/bin/terminal-rust", &[]), (false, false));
    }

    #[test]
    fn dash_c_takes_the_command_then_the_name_then_the_arguments() {
        assert_eq!(
            action(&["-c", "echo hi"]),
            command_string("echo hi", None, &[])
        );
        assert_eq!(
            action(&["-c", "x", "me"]),
            command_string("x", Some("me"), &[])
        );
        assert_eq!(
            action(&["-c", "x", "me", "-l", "two words"]),
            command_string("x", Some("me"), &["-l", "two words"])
        );
        assert_eq!(action(&["-lc", "x"]), command_string("x", None, &[]));
    }

    #[test]
    fn options_may_follow_dash_c_before_the_command() {
        let invocation = read("terminal_rust", &["-c", "-l", "--", "-i"]).expect("valid");

        assert!(invocation.login);
        assert!(!invocation.interactive);
        assert_eq!(invocation.action, command_string("-i", None, &[]));
    }

    #[test]
    fn dash_c_without_a_command_is_an_error() {
        for args in [&["-c"][..], &["-lc"], &["-c", "-i"], &["-c", "--"]] {
            assert_eq!(
                read("terminal_rust", args),
                Err(UsageError::MissingCommandString),
                "{args:?}"
            );
        }
    }

    #[test]
    fn dash_c_wins_over_a_file_of_the_same_name() {
        assert_eq!(
            action(&["-c", "existing"]),
            command_string("existing", None, &[])
        );
    }

    #[test]
    fn an_existing_file_is_a_script_with_its_arguments() {
        assert_eq!(
            run_of(&["existing", "-c", "a"]),
            script("existing", &["-c", "a"])
        );
        assert_eq!(run_of(&["--", "existing"]), script("existing", &[]));
        assert_eq!(run_of(&["-l", "existing"]), script("existing", &[]));
    }

    #[test]
    fn a_command_wins_over_a_file_of_the_same_name() {
        assert_eq!(run_of(&["both", "a"]), one_command(&["both", "a"]));
        assert_eq!(run_of(&["--", "both"]), one_command(&["both"]));
        assert_eq!(run_of(&["-il", "both"]), one_command(&["both"]));
    }

    #[test]
    fn a_path_is_never_taken_for_a_command() {
        assert_eq!(run_of(&["./both", "a"]), script("./both", &["a"]));
        assert_eq!(run_of(&["/bin/ls"]), script("/bin/ls", &[]));
    }

    #[test]
    fn an_operand_shaped_like_a_script_path_is_a_script_even_when_missing() {
        for path in ["./missing", "dir/missing", "missing.sh", "missing.trs"] {
            assert_eq!(run_of(&[path, "a"]), script(path, &["a"]), "{path}");
            assert_eq!(run_of(&["--", path]), script(path, &[]), "{path}");
        }
    }

    #[test]
    fn a_backslash_marks_a_path_on_windows_only() {
        let (missing, local) = if cfg!(windows) {
            (script(r"dir\missing", &[]), script(r".\both", &[]))
        } else {
            (one_command(&[r"dir\missing"]), one_command(&[r".\both"]))
        };

        assert_eq!(run_of(&[r"dir\missing"]), missing);
        assert_eq!(run_of(&[r".\both"]), local);
    }

    #[test]
    fn other_operands_are_one_command() {
        assert_eq!(run_of(&["ls", "a/b"]), one_command(&["ls", "a/b"]));
        assert_eq!(run_of(&["--", "ls"]), one_command(&["ls"]));
        assert_eq!(run_of(&["unknown"]), one_command(&["unknown"]));
        assert_eq!(action(&["-i", "echo", "-c"]), operands(&["echo", "-c"]));
    }

    #[test]
    fn an_unknown_option_starts_the_operands() {
        for args in [
            &["-x"][..],
            &["--foo", "-c"],
            &["-lx"],
            &["--help"],
            &["-"],
            &["--dir=a/b"],
        ] {
            assert_eq!(action(args), Action::OneLine(strings(args)), "{args:?}");
        }
        assert_eq!(flags("terminal_rust", &["-lx"]), (false, false));
    }

    #[test]
    fn after_the_end_of_options_a_dash_is_an_ordinary_character() {
        assert_eq!(run_of(&["--", "-dir/file"]), script("-dir/file", &[]));
        assert_eq!(run_of(&["--", "-c"]), one_command(&["-c"]));
    }

    #[test]
    fn text_arguments_are_kept_as_they_are() {
        let args = ["a", "b c"].map(OsString::from);

        assert_eq!(decode(args.into_iter()), Ok(strings(&["a", "b c"])));
    }

    #[cfg(unix)]
    #[test]
    fn an_argument_that_is_not_utf8_is_an_error() {
        use std::os::unix::ffi::OsStringExt;
        let args = [OsString::from("a"), OsString::from_vec(vec![0xff])];

        assert_eq!(
            decode(args.into_iter()),
            Err(UsageError::NotUnicode { position: 2 })
        );
    }
}
