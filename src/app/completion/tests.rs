use super::programs::ProgramCache;
#[cfg(unix)]
use super::programs::scan_path;
use super::*;
use crate::app::command;
use rustyline::Config;
use rustyline::history::{DefaultHistory, History};
use std::ffi::OsString;
use std::fs;
use std::path::MAIN_SEPARATOR;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn completions(prefix: &str) -> Vec<Pair> {
    let names: Vec<_> = command::command_names().map(str::to_string).collect();
    command_completions(&names, prefix)
}

#[test]
fn command_completion_matches_prefix() {
    let completions = completions("pw");

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replacement, "pwd ");
}

#[test]
fn clear_is_available_in_completion() {
    let completions = completions("cle");

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replacement, "clear ");
}

#[test]
fn dashboard_is_available_in_completion() {
    let completions = completions("dash");

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replacement, "dashboard ");
}

#[test]
fn git_is_available_in_completion() {
    let completions = completions("git");

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replacement, "git ");
}

#[test]
fn theme_is_available_in_completion() {
    let completions = completions("the");

    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replacement, "theme ");
}

#[test]
fn plugin_names_are_available_in_completion() {
    let completions = completions("plug");

    let replacements = completions
        .iter()
        .map(|completion| completion.replacement.as_str())
        .collect::<Vec<_>>();
    assert_eq!(replacements, ["plugins ", "plugin "]);
}

#[test]
fn custom_commands_are_available_in_completion() {
    let names = vec!["pwd".into(), "serve".into()];
    let completions = command_completions(&names, "ser");

    assert_eq!(completions[0].replacement, "serve ");
}

#[test]
fn history_hint_prefers_the_most_recent_prefix_and_ignores_duplicates() {
    let config = Config::builder()
        .history_ignore_dups(true)
        .expect("history configuration should be valid")
        .build();
    let mut history = DefaultHistory::with_config(&config);
    history.add("git status").expect("history accepts commands");
    history
        .add("git status")
        .expect("history accepts duplicates");
    history
        .add("git log --oneline")
        .expect("history accepts commands");
    let context = Context::new(&history);
    let helper = ShellHelper::with_command_names(Vec::new());

    assert_eq!(history.len(), 2);
    assert_eq!(
        helper.hint("git ", 4, &context).as_deref(),
        Some("log --oneline")
    );
}

#[test]
fn history_hint_handles_unicode_and_empty_history_without_logging() {
    let helper = ShellHelper::with_command_names(Vec::new());
    let empty = DefaultHistory::new();
    assert_eq!(helper.hint("cd ", 3, &Context::new(&empty)), None);

    let mut history = DefaultHistory::new();
    history
        .add("cd \"données locales\"")
        .expect("history accepts Unicode");
    let context = Context::new(&history);
    assert_eq!(
        helper.hint("cd \"d", 5, &context).as_deref(),
        Some("onnées locales\"")
    );
}

#[test]
fn history_hint_rejects_terminal_control_characters() {
    let helper = ShellHelper::with_command_names(Vec::new());
    let mut history = DefaultHistory::new();
    history
        .add("echo safe\u{1b}[31m")
        .expect("history accepts the fixture");

    assert_eq!(helper.hint("echo", 4, &Context::new(&history)), None);
}

#[test]
fn path_completion_preserves_quotes_spaces_and_native_separators() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "terminal-rust-completion-{}-{nonce}",
        std::process::id()
    ));
    let directory = root.join("données locales");
    fs::create_dir_all(&directory).expect("completion fixture should exist");

    let line = format!("cd \"{}{}données", root.display(), MAIN_SEPARATOR);
    let helper = ShellHelper::with_command_names(vec!["cd".into()]);
    let history = DefaultHistory::new();
    let (start, matches) = helper
        .complete(&line, line.len(), &Context::new(&history))
        .expect("path completion should succeed");

    assert_eq!(start, 4);
    assert_eq!(matches.len(), 1);
    assert!(matches[0].replacement.contains("données locales"));
    assert!(matches[0].replacement.contains(MAIN_SEPARATOR));
    assert!(matches[0].replacement.ends_with(MAIN_SEPARATOR));

    fs::remove_dir_all(root).expect("completion fixture should be removed");
}

#[test]
fn a_cursor_inside_a_character_is_moved_back_to_its_start() {
    let text = "caf\u{e9}";

    assert_eq!(floor_char_boundary(text, text.len()), text.len());
    assert_eq!(floor_char_boundary(text, 4), 3);
    assert_eq!(floor_char_boundary(text, 99), text.len());
    assert_eq!(floor_char_boundary("", 5), 0);
}

#[test]
fn completing_after_a_no_break_space_does_not_panic() {
    let history = DefaultHistory::new();
    let context = Context::new(&history);
    let helper = ShellHelper::with_command_names(vec!["ls".into()]);

    for line in ["ls\u{a0}", "ls\u{3000}", "\u{a0}", "caf\u{e9}\u{a0}"] {
        for pos in 0..=line.len() + 2 {
            // Positions inside a character must not panic either.
            let result = helper.complete(line, pos, &context);

            assert!(result.is_ok(), "{line:?} at {pos}");
        }
    }
}

#[test]
fn command_names_still_complete_at_the_start_of_the_line() {
    let history = DefaultHistory::new();
    let context = Context::new(&history);
    let helper =
        ShellHelper::with_command_names(vec!["pwd".into(), "ls".into()]).without_path_programs();

    let (start, pairs) = helper
        .complete("pw", 2, &context)
        .expect("completion should succeed");

    assert_eq!(start, 0);
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].replacement, "pwd ");
}

fn complete_at(helper: &ShellHelper, line: &str) -> (usize, Vec<String>) {
    let history = DefaultHistory::new();
    let context = Context::new(&history);
    let (start, pairs) = helper
        .complete(line, line.len(), &context)
        .expect("completion should succeed");
    (
        start,
        pairs.into_iter().map(|pair| pair.replacement).collect(),
    )
}

/// A helper whose list of `PATH` programs is fixed.
fn helper_with_programs(commands: &[&str], programs: &[&str]) -> ShellHelper {
    let helper = ShellHelper::with_command_names(commands.iter().map(|c| c.to_string()).collect());
    *helper.programs.cache.borrow_mut() = Some(ProgramCache {
        path: shell_env::value("PATH")
            .map(OsString::from)
            .unwrap_or_default(),
        loaded: Instant::now(),
        names: programs.iter().map(|p| p.to_string()).collect(),
    });
    helper
}

#[test]
fn programs_of_the_path_complete_next_to_the_built_in_commands() {
    let helper = helper_with_programs(&["git", "grep-builtin"], &["git", "gitk", "grep", "ls"]);

    let (start, names) = complete_at(&helper, "gi");

    assert_eq!(start, 0);
    // `git` is a built-in and a program: it is offered once.
    assert_eq!(names, ["git ", "gitk "]);
}

#[test]
fn programs_complete_after_a_pipe_or_a_separator() {
    let helper = helper_with_programs(&["echo"], &["grep", "gzip"]);

    assert_eq!(
        complete_at(&helper, "echo hi | gr"),
        (10, vec!["grep ".to_string()])
    );
    assert_eq!(
        complete_at(&helper, "echo hi && gz"),
        (11, vec!["gzip ".to_string()])
    );
}

#[test]
fn git_subcommands_and_references_complete_through_the_helper() {
    let guard = crate::test_support::cwd_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "terminal-rust-gitcompletion-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join(".git/refs/heads")).expect("fixture");
    fs::write(root.join(".git/refs/heads/feature-login"), "0\n").expect("fixture");
    fs::write(root.join("feature-file.txt"), "x").expect("fixture");
    let original = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("enter the fixture");

    let helper = helper_with_programs(&["git", "echo"], &[]);
    let subcommand = complete_at(&helper, "git sta");
    let reference = complete_at(&helper, "git checkout feature-l");
    let file = complete_at(&helper, "git add feature-f");
    let other = complete_at(&helper, "echo feature-l");

    // Leave the fixture and release the lock before asserting.
    std::env::set_current_dir(original).expect("restore cwd");
    fs::remove_dir_all(root).expect("fixture should be removed");
    drop(guard);

    assert_eq!(
        subcommand,
        (4, vec!["stash ".to_string(), "status ".to_string()])
    );
    assert_eq!(reference, (13, vec!["feature-login ".to_string()]));
    // `git add` takes files, and so does any other command.
    assert_eq!(file.1, vec!["feature-file.txt".to_string()]);
    assert!(other.1.is_empty(), "{other:?}");
}

#[test]
fn hidden_files_are_offered_only_after_a_dot() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should follow the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "terminal-rust-hidden-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join(".config")).expect("fixture");
    fs::write(root.join(".hidden"), "x").expect("fixture");
    fs::write(root.join("visible.txt"), "x").expect("fixture");
    let helper = helper_with_programs(&["cat"], &[]);
    let base = format!("{}{}", root.display(), MAIN_SEPARATOR);
    let names =
        |typed: &str| -> Vec<String> { complete_at(&helper, &format!("cat {base}{typed}")).1 };

    // Nothing typed: the visible file only.
    let all = names("");
    assert!(
        all.iter().any(|name| name.ends_with("visible.txt")),
        "{all:?}"
    );
    assert!(!all.iter().any(|name| name.contains(".hidden")), "{all:?}");
    assert!(!all.iter().any(|name| name.contains(".config")), "{all:?}");
    // A dot asks for them, and a prefix of a visible name still works.
    let dotted = names(".");
    assert!(
        dotted.iter().any(|name| name.contains(".hidden")),
        "{dotted:?}"
    );
    assert!(
        dotted.iter().any(|name| name.contains(".config")),
        "{dotted:?}"
    );
    assert!(
        !dotted.iter().any(|name| name.ends_with("visible.txt")),
        "{dotted:?}"
    );
    let visible = names("vis");
    assert_eq!(visible.len(), 1, "{visible:?}");

    fs::remove_dir_all(root).expect("fixture should be removed");
}

#[test]
fn arguments_still_complete_as_files() {
    let helper = helper_with_programs(&["echo"], &["grep"]);

    let (_, names) = complete_at(&helper, "echo gr");

    assert!(!names.iter().any(|name| name == "grep "), "{names:?}");
}

#[test]
fn no_prefix_does_not_list_every_program() {
    let helper = helper_with_programs(&["pwd"], &["aaa", "bbb"]);

    let (_, names) = complete_at(&helper, "");

    assert_eq!(names, ["pwd "]);
}

#[test]
fn a_path_like_word_is_not_a_command_name() {
    let helper = helper_with_programs(&["pwd"], &["pwdx"]);

    let (_, names) = complete_at(&helper, "./");

    assert!(
        names.iter().all(|name| name != "pwd " && name != "pwdx "),
        "{names:?}"
    );
}

#[test]
fn a_variable_reference_is_recognised_in_a_word() {
    assert_eq!(variable_reference("$HO"), Some((1, "HO", false)));
    assert_eq!(variable_reference("$"), Some((1, "", false)));
    assert_eq!(variable_reference("${HO"), Some((2, "HO", true)));
    assert_eq!(variable_reference("dir/$HO"), Some((5, "HO", false)));
    assert_eq!(variable_reference("\"$US"), Some((2, "US", false)));
    assert_eq!(variable_reference("plain"), None);
    assert_eq!(variable_reference("$A-b"), None);
    assert_eq!(variable_reference("$?"), None);
}

#[test]
fn variables_complete_with_or_without_braces() {
    let names = vec!["HOME".to_string(), "HOST".to_string(), "PATH".to_string()];

    let plain: Vec<_> = variable_completions(&names, "HO", false)
        .into_iter()
        .map(|pair| pair.replacement)
        .collect();
    let braced: Vec<_> = variable_completions(&names, "HOM", true)
        .into_iter()
        .map(|pair| pair.replacement)
        .collect();

    assert_eq!(plain, ["HOME", "HOST"]);
    assert_eq!(braced, ["HOME}"]);
}

#[test]
fn an_exported_variable_completes_after_a_dollar() {
    shell_env::export("TRS_COMPLETE_ME", "1");
    let helper = ShellHelper::with_command_names(Vec::new()).without_path_programs();

    let (start, names) = complete_at(&helper, "echo $TRS_COMPLETE_");

    assert_eq!(start, 6);
    assert_eq!(names, ["TRS_COMPLETE_ME"]);
}

#[cfg(unix)]
fn program_directory(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("terminal-rust-programs-{name}-{nonce}"));
    fs::create_dir_all(&directory).expect("fixture directory");
    directory
}

#[cfg(unix)]
#[test]
fn only_executable_files_of_the_path_are_programs() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let first = program_directory("first");
    let second = program_directory("second");
    for (directory, name, mode) in [
        (&first, "runme", 0o755),
        (&first, "notexec", 0o644),
        (&second, "other", 0o700),
        (&second, "runme", 0o755),
    ] {
        let file = directory.join(name);
        fs::write(&file, "#!/bin/sh\n").expect("fixture file");
        fs::set_permissions(&file, fs::Permissions::from_mode(mode)).expect("chmod");
    }
    fs::create_dir(first.join("subdir")).expect("fixture subdir");
    symlink(first.join("runme"), second.join("linked")).expect("symlink");
    let path = std::env::join_paths([&first, &second, &first.join("missing")]).expect("join");

    let names = scan_path(&path);

    assert_eq!(names, ["linked", "other", "runme"]);
    fs::remove_dir_all(first).expect("cleanup");
    fs::remove_dir_all(second).expect("cleanup");
}
