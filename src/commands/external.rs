use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[derive(Debug, Eq, PartialEq)]
pub enum RunError {
    /// No program of that name could be found.
    NotFound,
    /// The program could not start, or it ran and failed.
    Failed(String),
}

/// Run `name` with `args`, sharing the terminal streams, the environment and the
/// current directory of the shell.
pub fn run(name: &str, args: &[String]) -> Result<(), RunError> {
    let status = Command::new(resolve(name))
        .args(args)
        .status()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => RunError::NotFound,
            _ => RunError::Failed(format!("{name}: cannot start: {error}")),
        })?;

    if status.success() {
        Ok(())
    } else {
        Err(RunError::Failed(describe_exit(name, status)))
    }
}

/// Keep `Ctrl+C` from ending the shell while a program is running: the interrupt
/// reaches the foreground program, which stops, and the prompt comes back.
pub fn keep_shell_alive_on_interrupt() {
    // A second call reports an existing handler, which is the state we want.
    let _ = ctrlc::set_handler(|| {});
}

fn describe_exit(name: &str, status: ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("{name}: exited with status {code}"),
        None => format!("{name}: terminated by a signal"),
    }
}

#[cfg(windows)]
fn resolve(name: &str) -> OsString {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let pathext = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
    find_in_path(name, &path, &pathext, |candidate| candidate.is_file())
        .map(PathBuf::into_os_string)
        .unwrap_or_else(|| name.into())
}

#[cfg(not(windows))]
fn resolve(name: &str) -> OsString {
    name.into()
}

/// Search `path` for `name`, trying each `PATHEXT` extension in order.
///
/// A name with a path separator is never searched. The current directory is not
/// searched either, so a file dropped in a project cannot shadow a program.
#[cfg_attr(not(windows), allow(dead_code))]
fn find_in_path(
    name: &str,
    path: &OsStr,
    pathext: &OsStr,
    is_file: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if name.contains(['/', '\\']) {
        return None;
    }

    let extensions: Vec<&str> = pathext
        .to_str()
        .unwrap_or_default()
        .split(';')
        .filter(|extension| !extension.is_empty())
        .collect();
    let has_extension = Path::new(name).extension().is_some();

    for directory in std::env::split_paths(path) {
        if has_extension && is_file(&directory.join(name)) {
            return Some(directory.join(name));
        }
        for extension in &extensions {
            let candidate = directory.join(format!("{name}{extension}"));
            if is_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn search_path(directories: &[&str]) -> OsString {
        std::env::join_paths(directories).expect("test directories should join")
    }

    fn files<'a>(existing: &'a [&'a str]) -> impl Fn(&Path) -> bool + 'a {
        move |candidate| existing.iter().any(|file| Path::new(file) == candidate)
    }

    #[test]
    fn the_first_directory_wins() {
        let found = find_in_path(
            "tool",
            &search_path(&["/first", "/second"]),
            OsStr::new(".EXE"),
            files(&["/first/tool.EXE", "/second/tool.EXE"]),
        );

        assert_eq!(found, Some(PathBuf::from("/first/tool.EXE")));
    }

    #[test]
    fn extensions_are_tried_in_pathext_order() {
        let found = find_in_path(
            "npm",
            &search_path(&["/bin"]),
            OsStr::new(".COM;.EXE;.CMD"),
            files(&["/bin/npm.CMD", "/bin/npm.EXE"]),
        );

        assert_eq!(found, Some(PathBuf::from("/bin/npm.EXE")));
    }

    #[test]
    fn a_name_with_an_extension_is_tried_as_given() {
        let found = find_in_path(
            "build.bat",
            &search_path(&["/bin"]),
            OsStr::new(".EXE"),
            files(&["/bin/build.bat"]),
        );

        assert_eq!(found, Some(PathBuf::from("/bin/build.bat")));
    }

    #[test]
    fn a_name_with_a_separator_is_never_searched() {
        for name in ["./tool", "dir/tool", "dir\\tool"] {
            let found = find_in_path(name, &search_path(&["/bin"]), OsStr::new(".EXE"), |_| true);

            assert_eq!(found, None, "{name}");
        }
    }

    #[test]
    fn nothing_is_found_when_no_directory_has_the_program() {
        let found = find_in_path(
            "tool",
            &search_path(&["/bin", "/usr/bin"]),
            OsStr::new(".EXE"),
            |_| false,
        );

        assert_eq!(found, None);
    }

    #[test]
    fn a_missing_program_is_reported_as_not_found() {
        assert_eq!(
            run("terminal-rust-no-such-program", &[]),
            Err(RunError::NotFound)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_successful_program_returns_ok() {
        assert_eq!(run("true", &[]), Ok(()));
    }

    #[cfg(unix)]
    #[test]
    fn the_exit_status_of_a_failing_program_is_reported() {
        let arguments = ["-c".to_string(), "exit 3".to_string()];

        assert_eq!(
            run("sh", &arguments),
            Err(RunError::Failed("sh: exited with status 3".to_string()))
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_program_killed_by_a_signal_is_reported() {
        let arguments = ["-c".to_string(), "kill -9 $$".to_string()];

        assert_eq!(
            run("sh", &arguments),
            Err(RunError::Failed("sh: terminated by a signal".to_string()))
        );
    }

    #[cfg(windows)]
    #[test]
    fn the_exit_status_of_a_failing_program_is_reported() {
        let arguments = ["/C".to_string(), "exit 3".to_string()];

        assert_eq!(
            run("cmd", &arguments),
            Err(RunError::Failed("cmd: exited with status 3".to_string()))
        );
    }
}
