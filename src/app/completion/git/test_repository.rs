//! Real repositories for the completion tests, made by Git with a configuration
//! of their own: nothing of the user is read or written.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::path::temp_path;

/// A directory of its own, removed when dropped.
pub(super) struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub(super) fn new(name: &str) -> Self {
        let root = temp_path("gitrepo", name);
        fs::create_dir_all(root.join("home")).expect("sandbox");
        fs::write(root.join("gitconfig"), "").expect("sandbox");
        Self { root }
    }

    pub(super) fn path(&self) -> &Path {
        &self.root
    }

    /// Create the directory `relative` and give its path.
    pub(super) fn directory(&self, relative: &str) -> PathBuf {
        let directory = self.root.join(relative);
        fs::create_dir_all(&directory).expect("directory");
        directory
    }

    pub(super) fn file(&self, relative: &str) {
        fs::write(self.root.join(relative), "x").expect("file");
    }

    /// Run `git` in `directory`, which must succeed.
    pub(super) fn git(&self, directory: &Path, arguments: &[&str]) {
        let mut command = Command::new("git");
        // A `GIT_DIR` inherited from a hook would reach a real repository.
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let home = self.root.join("home");
        let output = command
            .args(arguments)
            .current_dir(directory)
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CONFIG_HOME", &home)
            .env("GIT_CONFIG_GLOBAL", self.root.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Test")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .output()
            .expect("git should be installed");
        assert!(
            output.status.success(),
            "git {arguments:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// A repository at `relative` with a commit on `main` and these branches.
    pub(super) fn repository(&self, relative: &str, branches: &[&str]) -> PathBuf {
        let directory = self.directory(relative);
        self.git(&directory, &["init", "-q", "-b", "main"]);
        self.git(
            &directory,
            &["commit", "-q", "--allow-empty", "-m", "initial"],
        );
        for branch in branches {
            self.git(&directory, &["branch", branch]);
        }
        directory
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
