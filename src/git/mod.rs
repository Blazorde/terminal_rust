use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Stdio};

/// Returns whether `path` belongs to a non-bare Git working tree.
///
/// A missing Git executable, an invalid path, and a directory outside a
/// repository are all treated as `false` so Git detection never prevents the
/// shell from starting.
pub fn is_inside_work_tree(path: &Path) -> bool {
    is_inside_work_tree_with_program(path, "git")
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
}
