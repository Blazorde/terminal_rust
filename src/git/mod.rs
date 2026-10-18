use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

const SUPPORTED_SUBCOMMANDS: [&str; 4] = ["help", "status", "branch", "log"];
const GIT_HELP: &str = "Terminal Rust Git helpers (read-only):\n\
  git status [options]  Show the working-tree status.\n\
  git branch [options]  List branches. Branch creation, deletion and renaming are not supported.\n\
  git log [options]     Show commit history. Writing output to a file is not supported.\n\
  git help              Show this help.\n\
\n\
Examples:\n\
  git status --short\n\
  git branch --all\n\
  git log -5 --oneline\n\
\n\
These helpers require Git to be installed for status, branch and log.\n\
Other Git subcommands, including push, reset and clean, are unavailable.\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptGitInfo {
    pub branch: String,
    pub dirty: bool,
}

pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status_code: Option<i32>,
    pub success: bool,
}

#[derive(Debug)]
pub enum GitCommandError {
    MissingSubcommand,
    HelpDoesNotAcceptArguments,
    UnsupportedSubcommand(String),
    UnsafeArgument(String),
    GitUnavailable(io::Error),
    CurrentDirectory(io::Error),
    Output(io::Error),
    CommandFailed(Option<i32>),
}

impl fmt::Display for GitCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSubcommand => write!(
                formatter,
                "git: missing subcommand (supported: {})",
                SUPPORTED_SUBCOMMANDS.join(", ")
            ),
            Self::HelpDoesNotAcceptArguments => write!(formatter, "git: usage is 'git help'"),
            Self::UnsupportedSubcommand(subcommand) => write!(
                formatter,
                "git: unsupported subcommand '{subcommand}' (supported: {})",
                SUPPORTED_SUBCOMMANDS.join(", ")
            ),
            Self::UnsafeArgument(argument) => {
                write!(
                    formatter,
                    "git: argument '{argument}' is not allowed in read-only mode"
                )
            }
            Self::GitUnavailable(error) => {
                write!(formatter, "git: executable unavailable: {error}")
            }
            Self::CurrentDirectory(error) => {
                write!(formatter, "git: unable to read current directory: {error}")
            }
            Self::Output(error) => write!(formatter, "git: unable to write output: {error}"),
            Self::CommandFailed(Some(code)) => {
                write!(formatter, "git: command failed with exit code {code}")
            }
            Self::CommandFailed(None) => write!(formatter, "git: command terminated by signal"),
        }
    }
}

impl std::error::Error for GitCommandError {}

/// Returns whether `path` belongs to a non-bare Git working tree.
///
/// A missing Git executable, an invalid path, and a directory outside a
/// repository are all treated as `false` so Git detection never prevents the
/// shell from starting.
pub fn is_inside_work_tree(path: &Path) -> bool {
    is_inside_work_tree_with_program(path, "git")
}

/// Collects the branch and working-tree state with one stable porcelain call.
///
/// Git being unavailable, a directory outside a repository, or malformed
/// output all disable the optional prompt segment instead of failing the shell.
pub fn prompt_info(path: &Path) -> Option<PromptGitInfo> {
    prompt_info_with_program(path, "git")
}

pub fn execute_developer_command(
    path: &Path,
    args: &[String],
) -> Result<GitCommandOutput, GitCommandError> {
    execute_developer_command_with_program(path, args, "git")
}

fn execute_developer_command_with_program(
    path: &Path,
    args: &[String],
    program: impl AsRef<OsStr>,
) -> Result<GitCommandOutput, GitCommandError> {
    let Some(subcommand) = args.first() else {
        return Err(GitCommandError::MissingSubcommand);
    };

    if subcommand == "help" {
        if args.len() != 1 {
            return Err(GitCommandError::HelpDoesNotAcceptArguments);
        }
        return Ok(GitCommandOutput {
            stdout: GIT_HELP.to_string(),
            stderr: String::new(),
            status_code: Some(0),
            success: true,
        });
    }

    if !SUPPORTED_SUBCOMMANDS.contains(&subcommand.as_str()) {
        return Err(GitCommandError::UnsupportedSubcommand(subcommand.clone()));
    }
    validate_read_only_args(subcommand, &args[1..])?;

    let output = Command::new(program)
        .arg("--no-pager")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(GitCommandError::GitUnavailable)?;

    Ok(GitCommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        status_code: output.status.code(),
        success: output.status.success(),
    })
}

fn validate_read_only_args(subcommand: &str, args: &[String]) -> Result<(), GitCommandError> {
    match subcommand {
        "branch" => {
            const MUTATING_FLAGS: [&str; 12] = [
                "-d",
                "-D",
                "-m",
                "-M",
                "-c",
                "-C",
                "--delete",
                "--move",
                "--copy",
                "--edit-description",
                "--set-upstream-to",
                "--unset-upstream",
            ];

            if let Some(argument) = args.iter().find(|argument| {
                !argument.starts_with('-')
                    || MUTATING_FLAGS.iter().any(|flag| {
                        argument.as_str() == *flag || argument.starts_with(&format!("{flag}="))
                    })
            }) {
                return Err(GitCommandError::UnsafeArgument(argument.clone()));
            }
        }
        "log" => {
            if let Some(argument) = args.iter().find(|argument| {
                argument.as_str() == "--output" || argument.starts_with("--output=")
            }) {
                return Err(GitCommandError::UnsafeArgument(argument.clone()));
            }
        }
        _ => {}
    }

    Ok(())
}

fn is_inside_work_tree_with_program(path: &Path, program: impl AsRef<OsStr>) -> bool {
    let output = Command::new(program)
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--is-inside-work-tree"])
        .stderr(Stdio::null())
        .output();

    let Ok(output) = output else {
        return false;
    };

    output.status.success() && output_means_inside(&output.stdout)
}

fn prompt_info_with_program(path: &Path, program: impl AsRef<OsStr>) -> Option<PromptGitInfo> {
    let output = Command::new(program)
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(path)
        .args([
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=normal",
        ])
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    parse_prompt_status(&output.stdout)
}

fn parse_prompt_status(stdout: &[u8]) -> Option<PromptGitInfo> {
    let output = std::str::from_utf8(stdout).ok()?;
    let mut head = None;
    let mut oid = None;
    let mut dirty = false;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("# branch.head ") {
            head = Some(value.trim());
        } else if let Some(value) = line.strip_prefix("# branch.oid ") {
            oid = Some(value.trim());
        } else if !line.starts_with("# ") && !line.is_empty() {
            dirty = true;
        }
    }

    let head = head?;
    let branch = if head == "(detached)" {
        let short_oid = oid
            .filter(|value| *value != "(initial)")
            .map(|value| &value[..value.len().min(7)])
            .unwrap_or("unknown");
        format!("detached@{short_oid}")
    } else {
        head.to_string()
    };

    Some(PromptGitInfo { branch, dirty })
}

fn output_means_inside(stdout: &[u8]) -> bool {
    std::str::from_utf8(stdout).is_ok_and(|output| output.trim() == "true")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::utils::path::temp_path;

    #[test]
    fn recognizes_git_inside_work_tree_output() {
        assert!(output_means_inside(b"true\n"));
        assert!(output_means_inside(b" true \r\n"));
    }

    #[test]
    fn rejects_non_work_tree_output() {
        assert!(!output_means_inside(b"false\n"));
        assert!(!output_means_inside(b"unexpected\n"));
        assert!(!output_means_inside(&[0xff]));
    }

    #[test]
    fn parses_clean_and_dirty_prompt_status() {
        let clean = parse_prompt_status(
            b"# branch.oid 0123456789abcdef\n# branch.head main\n# branch.upstream origin/main\n",
        );
        let dirty = parse_prompt_status(
            b"# branch.oid 0123456789abcdef\r\n# branch.head feature/test\r\n1 .M N... 100644 100644 100644 abc def file.txt\r\n? new.txt\r\n",
        );

        assert_eq!(
            clean,
            Some(PromptGitInfo {
                branch: "main".to_string(),
                dirty: false,
            })
        );
        assert_eq!(
            dirty,
            Some(PromptGitInfo {
                branch: "feature/test".to_string(),
                dirty: true,
            })
        );
    }

    #[test]
    fn gives_detached_head_a_stable_label() {
        let info =
            parse_prompt_status(b"# branch.oid 0123456789abcdef\n# branch.head (detached)\n");

        assert_eq!(
            info,
            Some(PromptGitInfo {
                branch: "detached@0123456".to_string(),
                dirty: false,
            })
        );
    }

    #[test]
    fn missing_git_disables_prompt_information() {
        let missing_program = temp_path("git", "missing-prompt-executable");

        assert!(prompt_info_with_program(Path::new("."), missing_program).is_none());
    }

    #[test]
    fn missing_git_is_treated_as_outside_repository() {
        let missing_program = temp_path("git", "missing-executable");

        assert!(!is_inside_work_tree_with_program(
            Path::new("."),
            &missing_program
        ));
    }

    #[test]
    fn detects_an_initialized_repository_when_git_is_available() {
        let directory = temp_path("git", "repository");
        fs::create_dir(&directory).expect("test repository should be created");

        let git_available = Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());

        if git_available {
            let initialized = Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&directory)
                .status()
                .expect("git should start");
            assert!(initialized.success());
            assert!(is_inside_work_tree(&directory));
        }

        fs::remove_dir_all(directory).expect("test repository should be removed");
    }

    #[test]
    fn directory_outside_git_is_not_a_work_tree() {
        let directory = temp_path("git", "outside");
        fs::create_dir(&directory).expect("test directory should be created");

        assert!(!is_inside_work_tree(&directory));

        fs::remove_dir(directory).expect("test directory should be removed");
    }

    #[test]
    fn developer_command_requires_a_supported_subcommand() {
        let missing = execute_developer_command(Path::new("."), &[]);
        assert!(matches!(missing, Err(GitCommandError::MissingSubcommand)));

        let unsupported = execute_developer_command(Path::new("."), &["push".to_string()]);
        assert!(matches!(
            unsupported,
            Err(GitCommandError::UnsupportedSubcommand(subcommand)) if subcommand == "push"
        ));
    }

    #[test]
    fn help_is_available_without_running_git() {
        let missing_program = temp_path("git", "missing-help-executable");
        let output = execute_developer_command_with_program(
            Path::new("."),
            &["help".to_string()],
            missing_program,
        )
        .expect("built-in help should not need the Git executable");

        assert!(output.success);
        assert_eq!(output.status_code, Some(0));
        assert!(output.stderr.is_empty());
        assert!(output.stdout.contains("git status [options]"));
        assert!(output.stdout.contains("git branch --all"));
        assert!(output.stdout.contains("git log -5 --oneline"));
        assert!(output.stdout.contains("push, reset and clean"));
    }

    #[test]
    fn help_rejects_extra_arguments() {
        let result =
            execute_developer_command(Path::new("."), &["help".to_string(), "status".to_string()]);

        assert!(matches!(
            result,
            Err(GitCommandError::HelpDoesNotAcceptArguments)
        ));
    }

    #[test]
    fn developer_commands_reject_mutating_arguments() {
        for args in [
            ["branch", "new-branch"].as_slice(),
            ["branch", "--delete"].as_slice(),
            ["log", "--output=history.txt"].as_slice(),
        ] {
            let args: Vec<String> = args.iter().map(ToString::to_string).collect();
            assert!(matches!(
                execute_developer_command(Path::new("."), &args),
                Err(GitCommandError::UnsafeArgument(_))
            ));
        }
    }

    #[test]
    fn developer_command_reports_a_missing_git_executable() {
        let missing_program = temp_path("git", "missing-command-executable");
        let result = execute_developer_command_with_program(
            Path::new("."),
            &["status".to_string()],
            &missing_program,
        );

        assert!(matches!(result, Err(GitCommandError::GitUnavailable(_))));
    }

    #[test]
    fn developer_status_runs_inside_an_initialized_repository() {
        let directory = temp_path("git", "developer-command-repository");
        fs::create_dir(&directory).expect("test repository should be created");

        let git_available = Command::new("git")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success());

        if git_available {
            let initialized = Command::new("git")
                .args(["init", "--quiet"])
                .current_dir(&directory)
                .status()
                .expect("git should start");
            assert!(initialized.success());

            let output = execute_developer_command(
                &directory,
                &["status".to_string(), "--short".to_string()],
            )
            .expect("git status should run");
            assert!(output.success);
            assert!(output.stderr.is_empty());
        }

        fs::remove_dir_all(directory).expect("test repository should be removed");
    }
}
