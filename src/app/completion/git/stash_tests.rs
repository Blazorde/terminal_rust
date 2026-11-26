//! The entries of the stash: where they are offered, how they are numbered and
//! what the line editor makes of them.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;

use rustyline::Context;
use rustyline::completion::{Completer, longest_common_prefix};
use rustyline::history::DefaultHistory;

use super::directory_tests::{offered, tab};
use super::repository::read_tail;
use super::stash::{self, MAX_ENTRIES, MAX_LOG_BYTES};
use super::test_repository::Sandbox;
use crate::app::completion::ShellHelper;

/// Stash a new file named `file`, with `message`.
fn push(sandbox: &Sandbox, directory: &Path, file: &str, message: &str) {
    fs::write(directory.join(file), file).expect("fixture");
    sandbox.git(directory, &["stash", "push", "-q", "-u", "-m", message]);
}

/// A repository with one entry for each of `files`, in that order.
fn stashed(sandbox: &Sandbox, relative: &str, files: &[&str]) -> PathBuf {
    let project = sandbox.repository(relative, &["topic"]);
    for file in files {
        push(sandbox, &project, file, file);
    }
    project
}

/// The names `git stash list` gives, in its order.
fn listed_by_git(sandbox: &Sandbox, directory: &Path) -> Vec<String> {
    sandbox
        .git_output(directory, &["stash", "list", "--format=%gd"])
        .lines()
        .map(str::to_string)
        .collect()
}

fn written(places: &[usize]) -> Vec<String> {
    places
        .iter()
        .map(|place| format!("'stash@{{{place}}}' "))
        .collect()
}

fn log_of(project: &Path) -> PathBuf {
    project.join(".git/logs/refs/stash")
}

/// The one line of the log of a repository with one entry.
fn only_line(project: &Path) -> Vec<u8> {
    let line = fs::read(log_of(project)).expect("log");
    assert_eq!(line.iter().filter(|&&byte| byte == b'\n').count(), 1);
    line
}

#[test]
fn the_actions_that_take_an_entry_offer_them() {
    let sandbox = Sandbox::new("stash-actions");
    let project = stashed(&sandbox, "project", &["a", "b"]);

    for line in [
        "git stash pop ",
        "git stash apply ",
        "git stash drop ",
        "git stash show ",
        "git stash show -p ",
        "git stash show --stat --include-untracked ",
        "git stash show -- ",
        "git stash pop --index -q ",
        "git stash drop --quiet ",
        "git stash apply --label-ours mine ",
        "git stash apply --label-theirs=theirs --label-base base ",
        "sudo git stash pop ",
        "git stash pop st",
        "git stash pop stash@{",
    ] {
        assert_eq!(offered(line, &project, None), written(&[0, 1]), "{line}");
    }
}

#[test]
fn one_entry_per_command_and_nothing_for_the_other_actions() {
    let sandbox = Sandbox::new("stash-elsewhere");
    let project = stashed(&sandbox, "project", &["a"]);

    for line in [
        "git stash ",
        "git stash p",
        "git stash push ",
        "git stash save ",
        "git stash list ",
        "git stash clear ",
        "git stash branch ",
        "git stash branch new ",
        "git stash pop stash@{0} ",
        "git stash show -p 'stash@{0}' ",
        "git stash show -S text ",
        // The value of this option is being typed.
        "git stash apply --label-ours ",
        "git stash -q pop ",
        "git stash show --",
        "git pop ",
    ] {
        assert!(offered(line, &project, None).is_empty(), "{line}");
    }
}

#[test]
fn a_word_that_opens_a_quote_keeps_it_and_is_closed() {
    let sandbox = Sandbox::new("stash-quoted");
    let project = stashed(&sandbox, "project", &["a", "b"]);
    let one = |word: &str| offered(&format!("git stash drop {word}"), &project, None);

    assert_eq!(one("'"), written(&[0, 1]));
    assert_eq!(one("'stash@{"), written(&[0, 1]));
    assert_eq!(one("'stash@{1"), written(&[1]));
    assert_eq!(one("\""), ["\"stash@{0}\" ", "\"stash@{1}\" "]);
    assert_eq!(one("\"stash@{1}"), ["\"stash@{1}\" "]);
    assert_eq!(one("stash@{1"), written(&[1]));
    assert!(one("'stash@{2").is_empty());
    assert!(one("'stash@{1}'").is_empty());
    assert!(one("'topic").is_empty());
    // The other names are not offered inside quotes.
    assert!(offered("git checkout 'to", &project, None).is_empty());
}

#[test]
fn the_places_are_those_of_git_stash_list() {
    let sandbox = Sandbox::new("stash-places");
    let project = stashed(&sandbox, "project", &["a", "b"]);
    // Git joins the lines of a message: one entry stays one line of the log.
    push(&sandbox, &project, "c", "two\nlines\n\nmore");
    sandbox.file("project/d");
    sandbox.git(&project, &["stash", "push", "-q", "-u"]);

    let all = listed_by_git(&sandbox, &project);
    assert_eq!(all.len(), 4);
    assert_eq!(stash::list(&project), all);

    sandbox.git(&project, &["stash", "drop", "-q", "stash@{1}"]);
    let left = listed_by_git(&sandbox, &project);
    assert_eq!(left.len(), 3);
    assert_eq!(stash::list(&project), left);

    sandbox.git(&project, &["stash", "clear"]);
    assert!(stash::list(&project).is_empty());
}

#[test]
fn a_message_that_is_not_text_and_a_line_not_ended_do_not_shift_the_places() {
    let sandbox = Sandbox::new("stash-bytes");
    let project = stashed(&sandbox, "project", &["a", "marker", "c"]);
    let mut log = fs::read(log_of(&project)).expect("log");
    let at = log
        .windows(6)
        .rposition(|window| window == b"marker")
        .expect("message");
    log[at] = 0xff;
    log.extend_from_slice(b"0000 being written");
    fs::write(log_of(&project), log).expect("log");

    let all = listed_by_git(&sandbox, &project);
    assert_eq!(all.len(), 3);
    assert_eq!(stash::list(&project), all);
}

#[test]
fn a_linked_worktree_and_a_directory_option_reach_the_shared_stash() {
    let sandbox = Sandbox::new("stash-worktree");
    let project = stashed(&sandbox, "project", &["a"]);
    sandbox.git(&project, &["worktree", "add", "-q", "../linked"]);
    let linked = sandbox.path().join("linked");
    push(&sandbox, &linked, "from-linked", "from-linked");
    let deep = sandbox.directory("linked/deep");

    assert_eq!(listed_by_git(&sandbox, &linked).len(), 2);
    assert_eq!(stash::list(&linked), listed_by_git(&sandbox, &linked));
    assert_eq!(stash::list(&deep), stash::list(&project));
    for line in [
        "git -C linked stash pop ",
        "git -C linked/deep stash show -p ",
    ] {
        assert_eq!(
            offered(line, sandbox.path(), None),
            written(&[0, 1]),
            "{line}"
        );
    }
    assert!(offered("git -C $NOWHERE stash pop ", &project, None).is_empty());
}

#[test]
fn only_the_latest_entries_are_offered() {
    let sandbox = Sandbox::new("stash-many");
    let project = stashed(&sandbox, "project", &["a"]);
    let line = only_line(&project);
    fs::write(log_of(&project), line.repeat(MAX_ENTRIES + 20)).expect("log");

    assert_eq!(listed_by_git(&sandbox, &project).len(), MAX_ENTRIES + 20);
    let latest: Vec<String> = (0..MAX_ENTRIES)
        .map(|place| format!("stash@{{{place}}}"))
        .collect();
    assert_eq!(stash::list(&project), latest);
}

#[test]
fn a_long_log_is_read_from_its_end() {
    let sandbox = Sandbox::new("stash-long");
    let project = stashed(&sandbox, "project", &["a"]);
    let line = only_line(&project);
    // The same entry with a message that makes its line a tenth of the limit.
    let mut long = line[..line.len() - 1].to_vec();
    long.resize(MAX_LOG_BYTES / 10 - 1, b'x');
    long.push(b'\n');
    fs::write(log_of(&project), long.repeat(25)).expect("log");

    let all = listed_by_git(&sandbox, &project);
    assert_eq!(all.len(), 25);
    assert_eq!(stash::list(&project), all[..10]);

    // A latest line longer than what is read: no place is made up.
    let mut huge = line[..line.len() - 1].to_vec();
    huge.resize(MAX_LOG_BYTES + 10, b'x');
    huge.push(b'\n');
    fs::write(log_of(&project), [line, huge].concat()).expect("log");
    assert!(stash::list(&project).is_empty());
}

#[test]
fn a_tail_loses_the_line_cut_by_the_limit_and_no_other() {
    let sandbox = Sandbox::new("stash-tail");
    let file = sandbox.path().join("file");
    fs::write(&file, "one\ntwo\nthree\n").expect("fixture");
    let tail = |limit| read_tail(&file, limit).expect("regular file");

    assert_eq!(tail(64), b"one\ntwo\nthree\n");
    assert_eq!(tail(14), b"one\ntwo\nthree\n");
    assert_eq!(tail(13), b"two\nthree\n");
    // The limit falls right on the start of a line.
    assert_eq!(tail(10), b"two\nthree\n");
    assert_eq!(tail(9), b"three\n");
    assert_eq!(tail(6), b"three\n");
    assert_eq!(tail(5), b"");
    assert_eq!(tail(0), b"");
    assert_eq!(read_tail(sandbox.path(), 16), None);
    assert_eq!(read_tail(&sandbox.path().join("missing"), 16), None);
}

#[cfg(unix)]
#[test]
fn a_fifo_instead_of_the_log_does_not_block() {
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let sandbox = Sandbox::new("stash-fifo");
    let project = stashed(&sandbox, "project", &["a"]);
    let log = log_of(&project);
    fs::remove_file(&log).expect("fixture");
    let made = Command::new("mkfifo").arg(&log).status().expect("mkfifo");
    assert!(made.success());

    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || sender.send(stash::list(&project)));
    let names = receiver
        .recv_timeout(Duration::from_secs(20))
        .expect("listing the stash should not block");

    assert!(names.is_empty());
}

/// The line after `Tab` at its end, changed as the list mode of the editor does it,
/// and what could be listed.
fn press_tab(line: &str) -> (String, Vec<String>) {
    let _guard = crate::test_support::cwd_lock()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let helper = ShellHelper::with_command_names(vec!["git".to_string()]).without_path_programs();
    let history = DefaultHistory::new();
    let (start, pairs) = helper
        .complete(line, line.len(), &Context::new(&history))
        .expect("completion should succeed");
    let mut changed = line.to_string();
    if let Some(common) = longest_common_prefix(&pairs)
        && (common.len() > line.len() - start || pairs.len() == 1)
    {
        changed.replace_range(start.., common);
    }
    (
        changed,
        pairs.into_iter().map(|pair| pair.display).collect(),
    )
}

#[test]
fn two_tabs_and_a_choice_write_a_quoted_entry() {
    let sandbox = Sandbox::new("stash-editor");
    let project = stashed(&sandbox, "with space", &["a", "b"]);
    let git = format!("git -C \"{}\" stash", project.display());
    let both = ["stash@{0}", "stash@{1}"];

    for (action, typed, quote) in [
        ("drop", "", '\''),
        ("show", "\"", '"'),
        ("pop", "stash@{", '\''),
    ] {
        let (line, _) = press_tab(&format!("{git} {action} {typed}"));
        assert_eq!(line, format!("{git} {action} {quote}stash@{{"));

        let (unchanged, listed) = press_tab(&line);
        assert_eq!(unchanged, line);
        assert_eq!(listed, both);

        let (chosen, _) = press_tab(&format!("{line}1"));
        assert_eq!(chosen, format!("{git} {action} {quote}stash@{{1}}{quote} "));
    }
}

#[test]
fn ten_entries_and_more_are_told_apart() {
    let sandbox = Sandbox::new("stash-ten");
    let project = stashed(&sandbox, "project", &["a"]);
    let line = only_line(&project);
    fs::write(log_of(&project), line.repeat(12)).expect("log");
    let git = format!("git -C \"{}\" stash apply", project.display());

    let (typed, listed) = press_tab(&format!("{git} 'stash@{{1"));
    assert_eq!(typed, format!("{git} 'stash@{{1"));
    assert_eq!(listed.len(), 3);
    for name in ["stash@{1}", "stash@{10}", "stash@{11}"] {
        assert!(listed.iter().any(|found| found == name), "{listed:?}");
    }
    let (chosen, _) = press_tab(&format!("{git} 'stash@{{11"));
    assert_eq!(chosen, format!("{git} 'stash@{{11}}' "));
}

#[test]
fn without_an_entry_the_file_names_are_left() {
    let sandbox = Sandbox::new("stash-none");
    let project = sandbox.repository("project", &["topic"]);
    sandbox.file("project/notes.txt");
    let quoted = format!("\"{}\"", project.display());

    assert!(stash::list(&project).is_empty());
    assert!(stash::list(&sandbox.directory("outside")).is_empty());
    assert_eq!(tab(&format!("git -C {quoted} stash pop no")), ["notes.txt"]);
    push(&sandbox, &project, "a", "a");
    assert_eq!(tab(&format!("git -C {quoted} stash pop S")), written(&[0]));
}
