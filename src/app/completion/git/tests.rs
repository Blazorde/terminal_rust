use std::fs;
use std::path::PathBuf;

use super::*;
use crate::utils::path::temp_path;

/// Complete `word` typed after `before_word`.
fn complete(before_word: &str, word: &str, directory: &Path) -> Option<Vec<Pair>> {
    let at = crate::app::completion::line::analyze(before_word);
    super::complete(&at.command, word, directory, Case::Exact)
}

fn displays(pairs: Option<Vec<Pair>>) -> Vec<String> {
    pairs
        .unwrap_or_default()
        .into_iter()
        .map(|pair| pair.display)
        .collect()
}

/// A repository with branches, a nested branch, a remote, tags and packed refs.
fn repository(name: &str) -> PathBuf {
    let root = temp_path("gitcomp", name);
    let git = root.join(".git");
    fs::create_dir_all(git.join("refs/heads/feature")).expect("fixture");
    fs::create_dir_all(git.join("refs/remotes/origin")).expect("fixture");
    fs::create_dir_all(git.join("refs/tags")).expect("fixture");
    for file in [
        "refs/heads/main",
        "refs/heads/develop",
        "refs/heads/feature/login",
        "refs/remotes/origin/main",
        "refs/remotes/origin/HEAD",
        "refs/tags/v1.0",
    ] {
        fs::write(git.join(file), "0000000000000000000000000000000000000000\n").expect("fixture");
    }
    fs::write(
        git.join("packed-refs"),
        "# pack-refs with: peeled fully-peeled sorted\n\
         1111111111111111111111111111111111111111 refs/heads/packed-branch\n\
         2222222222222222222222222222222222222222 refs/tags/v0.9\n\
         ^3333333333333333333333333333333333333333\n\
         4444444444444444444444444444444444444444 refs/remotes/origin/release\n\
         5555555555555555555555555555555555555555 refs/notes/ignored\n",
    )
    .expect("fixture");
    root
}

#[test]
fn subcommands_complete_after_git() {
    let root = temp_path("gitcomp", "none");
    fs::create_dir_all(&root).expect("fixture");
    assert_eq!(
        displays(complete("git ", "ch", &root)),
        vec!["checkout", "cherry-pick"]
    );
    assert_eq!(
        displays(complete("git ", "st", &root)),
        vec!["stash", "status"]
    );
    // Every subcommand is offered when nothing is typed.
    assert_eq!(
        displays(complete("git ", "", &root)).len(),
        SUBCOMMANDS.len()
    );
    assert!(complete("git ", "zzz", &root).is_none());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn the_subcommand_list_is_sorted_and_without_duplicates() {
    let mut sorted = SUBCOMMANDS.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, SUBCOMMANDS.to_vec());
}

#[test]
fn a_subcommand_is_followed_by_a_space() {
    let root = temp_path("gitcomp", "space");
    fs::create_dir_all(&root).expect("fixture");
    let pairs = complete("git ", "sta", &root).expect("matches");
    assert!(pairs.iter().all(|pair| pair.replacement.ends_with(' ')));
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn branches_remote_branches_and_tags_complete_after_checkout() {
    let root = repository("refs");
    let all = displays(complete("git checkout ", "", &root));
    assert_eq!(
        all,
        vec![
            "develop",
            "feature/login",
            "main",
            "origin/main",
            "origin/release",
            "packed-branch",
            "v0.9",
            "v1.0",
        ]
    );
    assert_eq!(
        displays(complete("git switch ", "fe", &root)),
        vec!["feature/login"]
    );
    assert_eq!(
        displays(complete("git merge ", "origin/", &root)),
        vec!["origin/main", "origin/release"]
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn a_nested_directory_of_the_repository_finds_the_references() {
    let root = repository("nested");
    let deep = root.join("src/app");
    fs::create_dir_all(&deep).expect("fixture");
    assert_eq!(
        displays(complete("git log ", "dev", &deep)),
        vec!["develop"]
    );
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn the_symbolic_head_of_a_remote_and_other_namespaces_are_not_offered() {
    let root = repository("head");
    let all = displays(complete("git checkout ", "", &root));
    assert!(!all.iter().any(|name| name.ends_with("HEAD")), "{all:?}");
    assert!(!all.iter().any(|name| name.contains("ignored")), "{all:?}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn a_reference_that_reads_as_an_option_is_not_offered() {
    let root = repository("dash");
    let git = root.join(".git");
    fs::write(git.join("refs/heads/-D"), "0\n").expect("fixture");
    fs::write(git.join("packed-refs"), "1 refs/tags/--force\n").expect("fixture");
    let all = displays(complete("git checkout ", "", &root));
    assert!(all.contains(&"main".to_string()), "{all:?}");
    assert!(!all.iter().any(|name| name.starts_with('-')), "{all:?}");
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn a_linked_worktree_reads_the_shared_references() {
    let root = repository("worktree");
    let linked = temp_path("gitcomp", "linked");
    let admin = root.join(".git/worktrees/linked");
    fs::create_dir_all(&admin).expect("fixture");
    fs::create_dir_all(&linked).expect("fixture");
    fs::write(admin.join("commondir"), "../..\n").expect("fixture");
    fs::write(
        linked.join(".git"),
        format!("gitdir: {}\n", admin.display()),
    )
    .expect("fixture");
    assert_eq!(
        displays(complete("git checkout ", "mai", &linked)),
        vec!["main"]
    );
    fs::remove_dir_all(root).expect("cleanup");
    fs::remove_dir_all(linked).expect("cleanup");
}

#[test]
fn other_commands_options_and_files_are_left_to_the_file_completion() {
    let root = repository("others");
    // Not git.
    assert!(complete("echo ", "ma", &root).is_none());
    // A subcommand whose arguments are files.
    assert!(complete("git add ", "ma", &root).is_none());
    assert!(complete("git rm ", "ma", &root).is_none());
    // An option.
    assert!(complete("git checkout ", "--", &root).is_none());
    assert!(complete("git ", "--ver", &root).is_none());
    // A quoted or escaped word matches no reference: it is a file name.
    assert!(complete("git checkout ", "'ma", &root).is_none());
    assert!(complete("git checkout ", "ma\\ n", &root).is_none());
    // Nothing matches: the file names take over.
    assert!(complete("git checkout ", "src/", &root).is_none());
    // Outside a repository.
    let elsewhere = temp_path("gitcomp", "outside");
    fs::create_dir_all(&elsewhere).expect("fixture");
    assert!(complete("git checkout ", "", &elsewhere).is_none());
    fs::remove_dir_all(root).expect("cleanup");
    fs::remove_dir_all(elsewhere).expect("cleanup");
}

#[test]
fn git_after_a_separator_is_recognised() {
    let root = repository("separator");
    assert_eq!(
        displays(complete("cd x && git checkout ", "dev", &root)),
        vec!["develop"]
    );
    assert_eq!(
        displays(complete("echo hi | git ", "sta", &root)),
        vec!["stash", "status"]
    );
    // The command before the separator is not git.
    assert!(complete("git status; echo ", "ma", &root).is_none());
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn a_missing_or_unreadable_repository_gives_nothing_without_failing() {
    let root = temp_path("gitcomp", "broken");
    fs::create_dir_all(root.join(".git")).expect("fixture");
    // A `.git` directory with no refs at all.
    assert!(complete("git checkout ", "", &root).is_none());
    // A `.git` file that points nowhere.
    let pointer = temp_path("gitcomp", "pointer");
    fs::create_dir_all(&pointer).expect("fixture");
    fs::write(pointer.join(".git"), "gitdir: /terminal-rust/no/such/dir\n").expect("fixture");
    assert!(complete("git checkout ", "", &pointer).is_none());
    fs::write(pointer.join(".git"), "garbage").expect("fixture");
    assert!(complete("git checkout ", "", &pointer).is_none());
    fs::remove_dir_all(root).expect("cleanup");
    fs::remove_dir_all(pointer).expect("cleanup");
}
