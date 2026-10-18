use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptGitInfo {
    pub branch: String,
    pub dirty: bool,
}

#[derive(Debug)]
pub enum GitCommandError {
    GitUnavailable(io::Error),
    CurrentDirectory(io::Error),
    CommandFailed(Option<i32>),
}

impl fmt::Display for GitCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GitUnavailable(error) => {
                write!(formatter, "git: executable unavailable: {error}")
            }
            Self::CurrentDirectory(error) => {
                write!(formatter, "git: unable to read current directory: {error}")
            }
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
) -> Result<ExitStatus, GitCommandError> {
    execute_developer_command_with_program(path, args, "git")
}

fn execute_developer_command_with_program(
    path: &Path,
    args: &[String],
    program: impl AsRef<OsStr>,
) -> Result<ExitStatus, GitCommandError> {
    Command::new(program)
        .current_dir(path)
        .args(args)
        // Inherit all three standard streams so Git can use the terminal for
        // pagers, editors, SSH prompts, credential helpers, and other tools.
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(GitCommandError::GitUnavailable)
        .and_then(|status| {
            if status.success() {
                Ok(status)
            } else {
                Err(GitCommandError::CommandFailed(status.code()))
            }
        })
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
    fn native_git_commands_run_inside_a_repository() {
        let directory = temp_path("git", "native-command-repository");
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

            let configured = Command::new("git")
                .args([
                    "-c",
                    "user.name=Terminal Rust",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "--allow-empty",
                    "--quiet",
                    "-m",
                    "fixture",
                ])
                .current_dir(&directory)
                .status()
                .expect("fixture commit should start");
            assert!(configured.success());

            let created = execute_developer_command(
                &directory,
                &["branch".to_string(), "native-command-test".to_string()],
            )
            .expect("native branch creation should run");
            assert!(created.success());

            let deleted = execute_developer_command(
                &directory,
                &[
                    "branch".to_string(),
                    "--delete".to_string(),
                    "native-command-test".to_string(),
                ],
            )
            .expect("native branch deletion should run");
            assert!(deleted.success());

            let failed =
                execute_developer_command(&directory, &["not-a-real-git-command".to_string()]);
            assert!(matches!(
                failed,
                Err(GitCommandError::CommandFailed(Some(_)))
            ));
        }

        fs::remove_dir_all(directory).expect("test repository should be removed");
    }
}
