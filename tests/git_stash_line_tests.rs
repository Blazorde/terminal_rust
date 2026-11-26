//! The lines that `Tab` writes for the entries of a stash, run by the shell on a
//! real repository: Git must receive the entry they name. The exact lines are
//! pinned by the completion tests (`git/stash_tests.rs`).

// Only part of the shared helpers is used here.
#[allow(dead_code)]
mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use common::{BINARY, Sandbox, finish, stderr, stdout};

/// `program` started with no Git setting of the machine: neither a configuration
/// of the user or of the system, nor an inherited `GIT_*` variable.
fn isolated(sandbox: &Sandbox, program: &str) -> Command {
    let global = sandbox.home().join("gitconfig");
    fs::write(&global, "").expect("the configuration should be written");
    let mut command = sandbox.command(program);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
        .env("GIT_CONFIG_GLOBAL", global)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid");
    command
}

fn succeeded(output: Output) -> String {
    assert!(output.status.success(), "{}", stderr(&output));
    stdout(&output)
}

struct Repository {
    sandbox: Sandbox,
    directory: PathBuf,
}

impl Repository {
    /// A repository whose stash holds the files `first`, `second` and `third`, one
    /// per entry.
    fn new(label: &str) -> Self {
        let sandbox = Sandbox::new(label);
        let directory = sandbox.work().join("with space");
        fs::create_dir_all(&directory).expect("the repository should be created");
        let repository = Self { sandbox, directory };
        repository.git(&["init", "-q", "-b", "main"]);
        repository.git(&["commit", "-q", "--allow-empty", "-m", "initial"]);
        for file in ["first", "second", "third"] {
            fs::write(repository.directory.join(file), file).expect("fixture");
            repository.git(&["stash", "push", "-q", "-u", "-m", file]);
        }
        repository
    }

    /// Run Git itself, without the shell.
    fn git(&self, arguments: &[&str]) -> String {
        let mut command = isolated(&self.sandbox, "git");
        command.arg("-C").arg(&self.directory).args(arguments);
        succeeded(finish(command, b""))
    }

    /// Run `git -C "<directory>" <rest>` as a line of the shell.
    fn shell_git(&self, rest: &str) -> String {
        let line = format!("git -C \"{}\" {rest}", self.directory.display());
        let mut command = isolated(&self.sandbox, BINARY);
        command.args(["-c", &line]);
        succeeded(finish(command, b""))
    }

    fn entries(&self) -> Vec<String> {
        self.git(&["stash", "list", "--format=%H"])
            .lines()
            .map(str::to_string)
            .collect()
    }
}

#[test]
fn git_receives_the_entry_that_the_completed_line_names() {
    let repository = Repository::new("stash line");
    let entries = repository.entries();
    assert_eq!(entries.len(), 3);

    for written in ["'stash@{1}' ", "\"stash@{1}\" "] {
        let object = repository.shell_git(&format!("rev-parse {written}"));
        assert_eq!(object.trim(), entries[1], "{written}");

        let shown = repository.shell_git(&format!(
            "stash show --include-untracked --name-only {written}"
        ));
        assert_eq!(shown.trim(), "second", "{written}");
    }
}

#[test]
fn a_completed_line_drops_the_entry_it_names() {
    for written in ["'stash@{1}' ", "\"stash@{1}\" "] {
        let repository = Repository::new("stash drop");
        let mut expected = repository.entries();
        expected.remove(1);

        repository.shell_git(&format!("stash drop {written}"));

        assert_eq!(repository.entries(), expected, "{written}");
    }
}
