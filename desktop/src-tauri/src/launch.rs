//! Launch options understood by the desktop app.
//!
//! Desktop environments, file managers and `x-terminal-emulator` start a
//! terminal with a working directory and, optionally, a command to run:
//!
//! ```text
//! terminal-rust-desktop --working-directory ~/code
//! terminal-rust-desktop -e htop
//! terminal-rust-desktop -w ~/code -- ls -la
//! ```
//!
//! Only the first session honors these options; extra tabs open a regular
//! shell. Unknown arguments are ignored because the webview toolkit may add
//! its own.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LaunchOptions {
    /// Directory the first session starts in.
    pub working_directory: Option<PathBuf>,
    /// Program and arguments to run instead of the Terminal Rust shell.
    pub command: Vec<OsString>,
}

static FIRST_SESSION: Mutex<Option<LaunchOptions>> = Mutex::new(None);

/// Parse process arguments, excluding the program name.
pub fn parse<I, S>(arguments: I) -> Result<LaunchOptions, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut options = LaunchOptions::default();
    let mut arguments = arguments.into_iter().map(Into::into);

    while let Some(argument) = arguments.next() {
        if argument == "-e" || argument == "--command" || argument == "--" {
            options.command = arguments.by_ref().collect();
            if options.command.is_empty() {
                return Err(format!(
                    "{} needs a command to run",
                    argument.to_string_lossy()
                ));
            }
            break;
        }
        if argument == "-w" || argument == "--working-directory" {
            let directory = arguments
                .next()
                .ok_or_else(|| format!("{} needs a directory", argument.to_string_lossy()))?;
            options.working_directory = Some(PathBuf::from(directory));
            continue;
        }
        if let Some(directory) = value_after(&argument, "--working-directory=") {
            options.working_directory = Some(PathBuf::from(directory));
        }
    }

    Ok(options)
}

fn value_after<'a>(argument: &'a OsStr, prefix: &str) -> Option<&'a OsStr> {
    argument
        .to_str()
        .and_then(|text| text.strip_prefix(prefix))
        .map(OsStr::new)
}

/// Read the options once at startup. Invalid options are reported on stderr
/// and replaced by the defaults, so a typo never prevents the window opening.
pub fn init_from_process_arguments() {
    let options = parse(std::env::args_os().skip(1)).unwrap_or_else(|message| {
        eprintln!("terminal-rust: ignoring launch options: {message}");
        LaunchOptions::default()
    });
    if let Ok(mut slot) = FIRST_SESSION.lock() {
        *slot = Some(options);
    }
}

/// Options for the first session. Later calls return the defaults.
pub fn take_for_first_session() -> LaunchOptions {
    FIRST_SESSION
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(arguments: &[&str]) -> Result<LaunchOptions, String> {
        parse(arguments.iter().copied())
    }

    fn os(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn no_arguments_gives_the_defaults() {
        assert_eq!(parsed(&[]), Ok(LaunchOptions::default()));
    }

    #[test]
    fn working_directory_accepts_the_three_spellings() {
        for arguments in [
            &["-w", "/srv"][..],
            &["--working-directory", "/srv"][..],
            &["--working-directory=/srv"][..],
        ] {
            assert_eq!(
                parsed(arguments).unwrap().working_directory,
                Some(PathBuf::from("/srv")),
                "{arguments:?}"
            );
        }
    }

    #[test]
    fn dash_e_takes_everything_after_it_as_the_command() {
        let options = parsed(&["-e", "htop", "-d", "5"]).unwrap();
        assert_eq!(options.command, os(&["htop", "-d", "5"]));
    }

    #[test]
    fn double_dash_and_long_form_also_start_the_command() {
        assert_eq!(
            parsed(&["--", "ls", "-la"]).unwrap().command,
            os(&["ls", "-la"])
        );
        assert_eq!(parsed(&["--command", "top"]).unwrap().command, os(&["top"]));
    }

    #[test]
    fn working_directory_and_command_combine() {
        let options = parsed(&["-w", "/tmp", "-e", "vim", "notes.txt"]).unwrap();
        assert_eq!(options.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(options.command, os(&["vim", "notes.txt"]));
    }

    #[test]
    fn options_after_the_command_belong_to_the_command() {
        let options = parsed(&["-e", "git", "-w", "x"]).unwrap();
        assert_eq!(options.working_directory, None);
        assert_eq!(options.command, os(&["git", "-w", "x"]));
    }

    #[test]
    fn unknown_arguments_are_ignored() {
        let options = parsed(&["--gtk-debug=all", "-w", "/srv", "--name", "x"]).unwrap();
        assert_eq!(options.working_directory, Some(PathBuf::from("/srv")));
        assert!(options.command.is_empty());
    }

    #[test]
    fn missing_values_are_reported() {
        assert!(parsed(&["-w"]).unwrap_err().contains("directory"));
        assert!(parsed(&["-e"]).unwrap_err().contains("command"));
        assert!(parsed(&["--"]).unwrap_err().contains("command"));
    }

    #[test]
    fn options_are_handed_to_the_first_session_only() {
        let expected = LaunchOptions {
            working_directory: Some(PathBuf::from("/srv")),
            command: os(&["htop"]),
        };
        *FIRST_SESSION.lock().unwrap() = Some(expected.clone());
        assert_eq!(take_for_first_session(), expected);
        assert_eq!(take_for_first_session(), LaunchOptions::default());
    }
}
