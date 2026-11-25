//! What a shell does before its first command: take the environment of a login shell,
//! set the colors of the native window and read `~/.terminal_rust_rc`.

use std::env;
use std::fs;
use std::io;
use std::process::ExitCode;

use crate::app::batch::{self, End};
use crate::config::{self, AppearanceSettings, appearance_ansi_sequences};
use crate::core::ShellEngine;
use crate::render;
use crate::render::printable::printable;
use crate::utils::path::home_directory;

const RC_FILE: &str = ".terminal_rust_rc";

/// Take the environment that `/etc/profile` and `~/.profile` set up for a login
/// shell. Windows has no such files, so nothing happens there.
pub(super) fn import_login_environment() {
    #[cfg(unix)]
    login::import_environment();
}

pub(super) fn apply_native_terminal_appearance() {
    if env::var_os("TERMINAL_RUST_NATIVE_UI").is_none() {
        return;
    }

    let mut settings = match config::load_appearance_settings() {
        Ok(settings) => settings,
        Err(error) => {
            let _ = render::stderr_line(format_args!("appearance: {error}; using defaults"));
            AppearanceSettings::default()
        }
    };
    let saved_settings = settings.clone();
    if let Err(error) = config::apply_appearance_environment(&mut settings) {
        let _ = render::stderr_line(format_args!(
            "appearance: {error}; ignoring invalid overrides"
        ));
        settings = saved_settings;
    }
    if let Err(error) = render::stdout(appearance_ansi_sequences(&settings)) {
        let _ = render::stderr_line(format_args!(
            "appearance: cannot apply terminal colors: {error}"
        ));
    }
}

/// Run the rc file in the current shell. `Some` is the status the shell must end
/// with, because the file called `exit`.
pub(super) fn run_rc(engine: &mut ShellEngine) -> Option<ExitCode> {
    // A relative home would run a file of whatever directory the shell starts in.
    let path = home_directory()
        .filter(|home| home.is_absolute())?
        .join(RC_FILE);
    if matches!(fs::metadata(&path), Err(error) if error.kind() == io::ErrorKind::NotFound) {
        return None;
    }
    let name = path.to_string_lossy();
    let content = match batch::read_script(&path) {
        Ok(content) => content,
        Err(error) => {
            let _ = render::stderr_line(format_args!(
                "terminal_rust: {}: {}",
                printable(&name),
                error.reason
            ));
            return None;
        }
    };

    let lines = content.lines().map(|line| Ok(line.to_string()));
    engine.enter_script();
    let end = batch::run_commands(engine, lines, Some(&name));
    engine.leave_script();
    match end {
        End::Exited => {
            let status = engine.exit_code();
            Some(batch::finish(engine, status))
        }
        // The error is already shown: the shell goes on to its prompt or its command.
        End::Completed | End::Failed(_) => None,
    }
}

/// The profiles are written for a POSIX shell, which this one is not: `/bin/sh` runs
/// them and hands back the environment they leave.
#[cfg(unix)]
mod login {
    use std::io::{self, Read};
    use std::process::{Command, Stdio};

    use crate::render;
    use crate::render::printable::printable;
    use crate::utils::shell_env;

    // The environment goes to descriptor 3, which the profiles do not get, so that
    // what they print or leave running cannot mix with it. The traps send it even when
    // a profile calls `exit`, fails or is interrupted. `TSTP` is ignored because a
    // stopped `sh` would leave the shell waiting for ever.
    const PROFILE_SCRIPT: &str = r#"
trs_emit() {
    trap - EXIT
    printf '\000TRS_ENV_BEGIN\000' >&3
    /usr/bin/env -0 >&3 && printf 'TRS_ENV_END\000' >&3
}
exec 3>&1 1>&2
trap trs_emit EXIT
trap 'exit 130' INT
trap '' TSTP
[ -r /etc/profile ] && . /etc/profile 3>&-
[ -r "$HOME/.profile" ] && . "$HOME/.profile" 3>&-
trs_emit
"#;

    const BEGIN: &[u8] = b"TRS_ENV_BEGIN";
    const END: &[u8] = b"TRS_ENV_END";
    const END_OF_OUTPUT: &[u8] = b"\0TRS_ENV_END\0";
    const MAX_OUTPUT_BYTES: usize = 8 * 1_048_576;
    /// They describe the `sh` process, not what the profiles set up.
    const OWN_TO_SH: [&str; 4] = ["PWD", "OLDPWD", "SHLVL", "_"];

    pub(super) fn import_environment() {
        let reason = match profile_output() {
            Ok(output) => match profile_changes(&output, shell_env::value) {
                Some(changes) => {
                    for (name, value) in changes {
                        shell_env::export(&name, &value);
                    }
                    return;
                }
                None => "/bin/sh gave no environment back".to_string(),
            },
            Err(error) => error.to_string(),
        };
        let _ = render::stderr_line(format_args!(
            "terminal_rust: cannot read the login profile: {}",
            printable(&reason)
        ));
    }

    // Not `job_control::run_foreground`: the output has to be read while `sh` runs,
    // and `sh` stays in the group of the shell so that `Ctrl+C` reaches it without
    // it ever becoming a job.
    fn profile_output() -> io::Result<Vec<u8>> {
        let mut command = Command::new("/bin/sh");
        command
            .args(["-c", PROFILE_SCRIPT])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        shell_env::prepare_child(&mut command);
        let mut child = command.spawn()?;
        let output = match child.stdout.take() {
            Some(pipe) => read_environment(pipe),
            None => Ok(Vec::new()),
        };
        if output.is_err() {
            let _ = child.kill();
        }
        let _ = child.wait();
        output
    }

    /// Read up to the end marker and not to the end of the pipe, which a program left
    /// running by a profile could keep open.
    fn read_environment(mut pipe: impl Read) -> io::Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut chunk = [0; 8192];
        while !output.ends_with(END_OF_OUTPUT) {
            match pipe.read(&mut chunk) {
                Ok(0) => break,
                Ok(count) => output.extend_from_slice(&chunk[..count]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
            if output.len() > MAX_OUTPUT_BYTES {
                return Err(io::Error::other("the environment is too large"));
            }
        }
        Ok(output)
    }

    /// The variables of `output` that `current` does not already have with the same
    /// value. `None` when the markers are not both there.
    fn profile_changes(
        output: &[u8],
        current: impl Fn(&str) -> Option<String>,
    ) -> Option<Vec<(String, String)>> {
        let mut fields = output.split(|byte| *byte == 0);
        fields.find(|field| *field == BEGIN)?;
        let mut changes = Vec::new();
        for field in fields {
            if field == END {
                return Some(changes);
            }
            let Some((name, value)) = std::str::from_utf8(field)
                .ok()
                .and_then(|entry| entry.split_once('='))
            else {
                continue;
            };
            if shell_env::is_valid_name(name)
                && !OWN_TO_SH.contains(&name)
                && current(name).as_deref() != Some(value)
            {
                changes.push((name.to_string(), value.to_string()));
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use std::io::{self, Read};

        use super::{profile_changes, read_environment};

        /// A pipe that a running program keeps open: it never reaches its end.
        struct StillOpen(std::vec::IntoIter<&'static [u8]>);

        impl Read for StillOpen {
            fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
                let chunk = self
                    .0
                    .next()
                    .ok_or_else(|| io::Error::other("this read would block"))?;
                buffer[..chunk.len()].copy_from_slice(chunk);
                Ok(chunk.len())
            }
        }

        #[test]
        fn reading_stops_at_the_end_marker() {
            let chunks: Vec<&[u8]> = vec![b"\0TRS_ENV_BEGIN\0A=1\0TRS_", b"ENV_END\0"];

            let output = read_environment(StillOpen(chunks.into_iter())).expect("reads");

            assert_eq!(output, b"\0TRS_ENV_BEGIN\0A=1\0TRS_ENV_END\0");
        }

        fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
            expected
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect()
        }

        #[test]
        fn values_keep_their_lines_and_equal_signs() {
            let output = b"noise\0TRS_ENV_BEGIN\0A=one\ntwo\0B=x=y=\0EMPTY=\0TRS_ENV_END\0";

            assert_eq!(
                profile_changes(output, |_| None),
                Some(pairs(&[("A", "one\ntwo"), ("B", "x=y="), ("EMPTY", "")]))
            );
        }

        #[test]
        fn only_new_or_changed_variables_with_valid_names_are_kept() {
            let output = b"\0TRS_ENV_BEGIN\0SAME=1\0CHANGED=2\0NEW=3\0PWD=/p\0OLDPWD=/o\0\
SHLVL=4\0_=/usr/bin/env\0BASH_FUNC_f%%=() { :; }\0no-equals\0BAD=\xff\0TRS_ENV_END\0";
            let current = |name: &str| match name {
                "SAME" => Some("1".to_string()),
                "CHANGED" => Some("1".to_string()),
                _ => None,
            };

            assert_eq!(
                profile_changes(output, current),
                Some(pairs(&[("CHANGED", "2"), ("NEW", "3")]))
            );
        }

        #[test]
        fn output_without_both_markers_gives_nothing() {
            for output in [
                &b""[..],
                b"A=1\0",
                b"\0TRS_ENV_BEGIN\0A=1\0",
                b"A=1\0TRS_ENV_END\0",
                b"\0TRS_ENV_BEGIN\0A=TRS_ENV_END\0",
            ] {
                assert_eq!(profile_changes(output, |_| None), None, "{output:?}");
            }
        }
    }
}
