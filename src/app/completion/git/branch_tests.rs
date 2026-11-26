//! `git branch -d`: which options ask for a deletion and which branches Git can
//! then be given.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::directory_tests::{offered, tab};
use super::references;
use super::test_repository::Sandbox;
use crate::app::history;
use crate::app::parser::parse_words;

/// A repository on `main` with local branches, remote branches, the symbolic
/// `HEAD` of a remote and tags that start like the branches.
fn with_branches(sandbox: &Sandbox, relative: &str) -> PathBuf {
    let project = sandbox.repository(relative, &["topic", "feature/login"]);
    for name in ["refs/remotes/origin/main", "refs/remotes/origin/topic"] {
        sandbox.git(&project, &["update-ref", name, "HEAD"]);
    }
    sandbox.git(
        &project,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    for tag in ["topic-v1", "feature/tagged", "origin/tagged"] {
        sandbox.git(&project, &["tag", tag]);
    }
    project
}

const LOCAL: [&str; 2] = ["feature/login ", "topic "];
const REMOTE: [&str; 2] = ["origin/main ", "origin/topic "];

/// The words the parser reads from the candidates, as the shell would pass them to
/// Git.
fn read_back(replacements: &[String]) -> Vec<String> {
    replacements
        .iter()
        .map(|replacement| {
            assert_eq!(history::expand(replacement, &[]), Ok(None), "{replacement}");
            let words = parse_words(replacement, &|_| Some("EXPANDED".to_string()))
                .expect("the quoting should parse");
            assert_eq!(words.len(), 1, "{replacement}");
            assert!(words[0].pattern.is_none(), "{replacement}");
            words[0].text.clone()
        })
        .collect()
}

/// Give every candidate of `line` to `git branch <options>`, which must accept it.
fn delete_all(sandbox: &Sandbox, directory: &Path, line: &str, options: &[&str]) -> Vec<String> {
    let names = read_back(&offered(line, directory, None));
    for name in &names {
        let mut arguments = vec!["branch"];
        arguments.extend_from_slice(options);
        arguments.push(name);
        sandbox.git(directory, &arguments);
    }
    names
}

#[test]
fn every_way_to_ask_for_a_deletion_offers_the_other_local_branches() {
    let sandbox = Sandbox::new("branch-local");
    let project = with_branches(&sandbox, "project");

    for line in [
        "git branch -d ",
        "git branch -D ",
        "git branch --delete ",
        "git branch -fd ",
        "git branch -df ",
        "git branch -qd ",
        "git branch -vD ",
        "git branch -id ",
        "git branch -d -f ",
        "git branch --force --delete ",
        "git branch --sort refname -d ",
        "git branch -d -- ",
        "git branch -d topic ",
        "git branch -D topic other ",
        "git branch topic -d ",
        "git branch --no-delete -d ",
        "sudo git branch -d ",
    ] {
        assert_eq!(offered(line, &project, None), LOCAL, "{line}");
    }
    assert_eq!(offered("git branch -d to", &project, None), ["topic "]);
}

#[test]
fn a_deletion_of_remote_branches_offers_only_those() {
    let sandbox = Sandbox::new("branch-remote");
    let project = with_branches(&sandbox, "project");

    for line in [
        "git branch -d -r ",
        "git branch -r -d ",
        "git branch -dr ",
        "git branch -rd ",
        "git branch -Dr ",
        "git branch -rD ",
        "git branch -r --delete ",
        "git branch --remotes -D ",
        "git branch --delete --remotes ",
        "git branch -d -r origin/main ",
        "git branch -d origin/main -r ",
        "git branch -dr -- ",
    ] {
        assert_eq!(offered(line, &project, None), REMOTE, "{line}");
    }
}

#[test]
fn without_a_deletion_every_reference_is_still_offered() {
    let sandbox = Sandbox::new("branch-kept");
    let project = with_branches(&sandbox, "project");
    let all = offered("git checkout ", &project, None);
    assert!(all.contains(&"main ".to_string()) && all.contains(&"topic-v1 ".to_string()));

    for line in [
        "git branch ",
        "git branch -r ",
        "git branch -m ",
        "git branch --remotes ",
        // After `--` these are names.
        "git branch -- -d ",
        "git branch -- --delete ",
        // These are values.
        "git branch --sort -d ",
        "git branch --format --delete ",
        "git branch -u -d ",
        "git branch -udr ",
        "git branch -tdirect ",
        "git branch -d --no-delete ",
        // Another action, and other commands.
        "git branch --delete-merged main ",
        "git checkout -d ",
        "git log -d ",
    ] {
        assert_eq!(offered(line, &project, None), all, "{line}");
    }
    assert!(offered("git branch -d --re", &project, None).is_empty());
}

#[test]
fn git_accepts_every_branch_offered() {
    let sandbox = Sandbox::new("branch-accepted");
    let project = with_branches(&sandbox, "project");
    sandbox.git(&project, &["tag", "topic"]);

    assert_eq!(
        delete_all(&sandbox, &project, "git branch -d ", &["-d"]),
        ["feature/login", "topic"]
    );
    assert_eq!(
        delete_all(&sandbox, &project, "git branch -dr ", &["-d", "-r"]),
        ["origin/main", "origin/topic"]
    );
    assert!(offered("git branch -d ", &project, None).is_empty());
    assert!(offered("git branch -dr ", &project, None).is_empty());
}

#[test]
fn packed_branches_are_offered_and_the_current_one_is_still_left_out() {
    let sandbox = Sandbox::new("branch-packed");
    let project = with_branches(&sandbox, "project");
    sandbox.git(&project, &["checkout", "-q", "-b", "feature/current"]);
    sandbox.git(&project, &["pack-refs", "--all"]);
    sandbox.git(&project, &["branch", "loose"]);
    assert!(!project.join(".git/refs/heads/topic").exists());
    assert!(!project.join(".git/refs/heads/feature/current").exists());

    assert_eq!(
        references::remote_branches(&project),
        ["origin/main", "origin/topic"]
    );
    assert_eq!(
        delete_all(&sandbox, &project, "git branch -d ", &["-d"]),
        ["feature/login", "loose", "main", "topic"]
    );
}

#[test]
fn a_linked_worktree_leaves_out_its_own_branch() {
    let sandbox = Sandbox::new("branch-worktree");
    let project = sandbox.repository("project", &["topic"]);
    sandbox.git(
        &project,
        &["worktree", "add", "-q", "-b", "linked-branch", "../linked"],
    );
    let linked = sandbox.path().join("linked");
    let deep = sandbox.directory("linked/deep");

    // `main` is checked out in the other working tree: that is not looked at.
    for directory in [&linked, &deep] {
        assert_eq!(references::local_branches(directory), ["main", "topic"]);
    }
    assert_eq!(
        references::local_branches(&project),
        ["linked-branch", "topic"]
    );
    for line in ["git -C linked branch -d ", "git -C linked/deep branch -D "] {
        assert_eq!(
            offered(line, sandbox.path(), None),
            ["main ", "topic "],
            "{line}"
        );
    }
    sandbox.git(&linked, &["branch", "-d", "topic"]);
    assert_eq!(references::local_branches(&linked), ["main"]);
}

#[test]
fn with_a_detached_head_every_local_branch_can_go() {
    let sandbox = Sandbox::new("branch-detached");
    let project = with_branches(&sandbox, "project");
    sandbox.git(&project, &["checkout", "-q", "--detach"]);

    assert_eq!(
        delete_all(&sandbox, &project, "git branch -d ", &["-d"]),
        ["feature/login", "main", "topic"]
    );
}

#[test]
fn a_hostile_branch_is_neither_run_nor_read_as_an_option() {
    let sandbox = Sandbox::new("branch-hostile");
    let names = [
        "$(x)", "$HOME", "a!b", "a;b", "a`b", "o'q", "{x,y}", "#note",
    ];
    let project = sandbox.repository("project", &names);
    // `git branch` refuses to make it, a ref can still be written.
    sandbox.git(&project, &["update-ref", "refs/heads/-D", "HEAD"]);

    let replacements = tab(&format!("git -C \"{}\" branch -d ", project.display()));
    let mut found = read_back(&replacements);
    found.sort();
    let mut expected = names.map(str::to_string);
    expected.sort();

    assert_eq!(found, expected);
    for name in &found {
        sandbox.git(&project, &["branch", "-d", name]);
    }
}

#[test]
fn without_a_branch_to_delete_the_file_names_are_left() {
    let sandbox = Sandbox::new("branch-files");
    let project = with_branches(&sandbox, "project");
    sandbox.file("project/notes.txt");
    sandbox.file("project/mainly.txt");
    let git = format!("git -C \"{}\" branch", project.display());
    let after = |rest: &str| tab(&format!("{git} {rest}"));

    assert_eq!(after("-d TOP"), ["topic "]);
    assert_eq!(after("-dr ORIGIN/T"), ["origin/topic "]);
    assert_eq!(after("-d no"), ["notes.txt"]);
    // Only the current branch and a file start like this.
    assert_eq!(after("-d mai"), ["mainly.txt"]);
    assert_eq!(after("-d --del"), ["--delete "]);
    assert!(references::local_branches(&sandbox.directory("outside")).is_empty());
    fs::remove_file(project.join(".git/HEAD")).expect("fixture");
    assert_eq!(
        references::local_branches(&project),
        ["feature/login", "main", "topic"]
    );
}

#[test]
fn the_value_of_an_option_is_not_read_as_a_deletion() {
    let sandbox = Sandbox::new("branch-valued");
    let project = with_branches(&sandbox, "project");
    let all = offered("git checkout ", &project, None);

    // Spelled out rather than read from the table, so that losing an entry shows.
    for option in [
        "-u",
        "--contains",
        "--delete-merged",
        "--forked",
        "--format",
        "--merged",
        "--no-contains",
        "--no-merged",
        "--points-at",
        "--set-upstream-to",
        "--sort",
    ] {
        for value in ["-d", "-D", "--delete"] {
            let line = format!("git branch {option} {value} ");
            assert_eq!(offered(&line, &project, None), all, "{line}");
        }
    }
}

#[test]
fn no_delete_does_not_take_back_a_forced_deletion() {
    let sandbox = Sandbox::new("branch-no-delete");
    let project = with_branches(&sandbox, "project");

    for line in [
        "git branch -D --no-delete ",
        "git branch --no-delete -D ",
        "git branch -d -D --no-delete ",
        "git branch -dD --no-delete ",
    ] {
        assert_eq!(offered(line, &project, None), LOCAL, "{line}");
    }
    assert_eq!(
        offered("git branch -Dr --no-delete ", &project, None),
        REMOTE
    );
    assert_eq!(
        delete_all(
            &sandbox,
            &project,
            "git branch -D --no-delete ",
            &["-D", "--no-delete"]
        ),
        ["feature/login", "topic"]
    );
    assert_eq!(references::local_branches(&project), [] as [&str; 0]);
}

#[cfg(unix)]
#[test]
fn a_head_that_is_a_link_still_leaves_out_its_branch() {
    let sandbox = Sandbox::new("branch-link-head");
    let project = sandbox.repository("project", &["topic", "feature/login"]);
    sandbox.git(&project, &["config", "core.preferSymlinkRefs", "true"]);
    sandbox.git(&project, &["switch", "-q", "feature/login"]);
    let head = project.join(".git/HEAD");
    assert!(head.is_symlink());

    assert_eq!(references::local_branches(&project), ["main", "topic"]);
    // The link then points at no file.
    sandbox.git(&project, &["pack-refs", "--all"]);
    assert!(head.is_symlink() && !head.exists());
    assert_eq!(
        delete_all(&sandbox, &project, "git branch -d ", &["-d"]),
        ["main", "topic"]
    );
}

#[test]
fn the_current_branch_is_matched_without_case_only_where_files_are() {
    let names = || BTreeSet::from(["main", "topic", "topic-v1"].map(str::to_string));
    let left = |current: &str, ignore_case: bool| {
        let mut left = names();
        references::remove_current(&mut left, current, ignore_case);
        left.into_iter().collect::<Vec<_>>()
    };

    assert_eq!(left("Topic", true), ["main", "topic-v1"]);
    assert_eq!(left("Topic", false), ["main", "topic", "topic-v1"]);
    for ignore_case in [true, false] {
        assert_eq!(left("topic", ignore_case), ["main", "topic-v1"]);
    }

    let sandbox = Sandbox::new("branch-case");
    let project = sandbox.repository("project", &["topic"]);
    // What `git checkout Topic` leaves in `HEAD` on those systems.
    sandbox.git(&project, &["symbolic-ref", "HEAD", "refs/heads/Topic"]);
    let expected: &[&str] = if cfg!(any(windows, target_os = "macos")) {
        &["main"]
    } else {
        &["main", "topic"]
    };
    assert_eq!(references::local_branches(&project), expected);
}
