use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::utils::shell_env;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

/// How long the list of programs found in `PATH` is reused. Reading every
/// directory of `PATH` takes a moment, and `Tab` is pressed many times in a row.
const PROGRAM_CACHE_TTL: Duration = Duration::from_secs(10);

struct ProgramCache {
    path: OsString,
    loaded: Instant,
    names: Vec<String>,
}

pub struct ShellHelper {
    filename_completer: FilenameCompleter,
    history_hinter: HistoryHinter,
    command_names: Vec<String>,
    search_path: bool,
    programs: RefCell<Option<ProgramCache>>,
}

impl ShellHelper {
    pub fn with_command_names(command_names: Vec<String>) -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
            history_hinter: HistoryHinter::new(),
            command_names,
            search_path: true,
            programs: RefCell::new(None),
        }
    }

    /// Do not offer the programs of `PATH`: for tests that must not depend on
    /// what is installed.
    #[cfg(test)]
    fn without_path_programs(mut self) -> Self {
        self.search_path = false;
        self
    }

    /// The programs of `PATH` that start with `prefix`, from a cache that is
    /// rebuilt when `PATH` changes or after a few seconds.
    fn programs_starting_with(&self, prefix: &str) -> Vec<String> {
        if !self.search_path {
            return Vec::new();
        }
        let path = shell_env::value("PATH")
            .map(OsString::from)
            .unwrap_or_default();
        let mut cache = self.programs.borrow_mut();
        let stale = cache.as_ref().is_none_or(|cached| {
            cached.path != path || cached.loaded.elapsed() > PROGRAM_CACHE_TTL
        });
        if stale {
            let names = scan_path(&path);
            *cache = Some(ProgramCache {
                path,
                loaded: Instant::now(),
                names,
            });
        }
        cache
            .as_ref()
            .map(|cached| {
                cached
                    .names
                    .iter()
                    .filter(|name| name.starts_with(prefix))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// The names of the executable files in the directories of `path`, sorted. On
/// Windows the extension that makes a file runnable (`.exe`) is left out, as the
/// shell finds the program without it.
fn scan_path(path: &OsStr) -> Vec<String> {
    let mut names = BTreeSet::new();
    for directory in std::env::split_paths(path) {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if let Some(program) = program_name(&entry.path(), &name) {
                names.insert(program);
            }
        }
    }
    names.into_iter().collect()
}

#[cfg(unix)]
fn program_name(path: &Path, name: &str) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    // `metadata` follows symbolic links, and many programs are links.
    let metadata = fs::metadata(path).ok()?;
    (metadata.is_file() && metadata.permissions().mode() & 0o111 != 0).then(|| name.to_string())
}

#[cfg(windows)]
fn program_name(path: &Path, name: &str) -> Option<String> {
    const EXTENSIONS: [&str; 4] = [".exe", ".cmd", ".bat", ".com"];
    let lower = name.to_ascii_lowercase();
    let extension = EXTENSIONS
        .iter()
        .find(|extension| lower.ends_with(**extension))?;
    fs::metadata(path)
        .ok()?
        .is_file()
        .then(|| name[..name.len() - extension.len()].to_string())
}

/// Whether the word that starts at `word_start` is where a command goes: at the
/// start of the line, or after `;`, `|` or `&` (so after `&&` and `||` too).
fn is_command_position(before_cursor: &str, word_start: usize) -> bool {
    let before_word = before_cursor[..word_start].trim_end();
    before_word.is_empty() || before_word.ends_with([';', '|', '&'])
}

/// For a word that ends inside a variable reference (`$HO`, `${HO`), where the
/// name starts in the word, the part typed so far, and whether it is `${...}`.
fn variable_reference(word: &str) -> Option<(usize, &str, bool)> {
    let dollar = word.rfind('$')?;
    let after = &word[dollar + 1..];
    let (offset, prefix, braced) = match after.strip_prefix('{') {
        Some(rest) => (dollar + 2, rest, true),
        None => (dollar + 1, after, false),
    };
    prefix
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
        .then_some((offset, prefix, braced))
}

fn variable_completions(names: &[String], prefix: &str, braced: bool) -> Vec<Pair> {
    names
        .iter()
        .filter(|name| name.starts_with(prefix))
        .map(|name| Pair {
            display: name.clone(),
            replacement: if braced {
                format!("{name}}}")
            } else {
                name.clone()
            },
        })
        .collect()
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let pos = floor_char_boundary(line, pos);
        let before_cursor = &line[..pos];
        let command_start = word_start(before_cursor);
        let current_word = &before_cursor[command_start..];

        if let Some((offset, prefix, braced)) = variable_reference(current_word) {
            let names: Vec<String> = shell_env::vars()
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            return Ok((
                command_start + offset,
                variable_completions(&names, prefix, braced),
            ));
        }

        let looks_like_a_path =
            current_word.contains(['/', '\\']) || current_word.starts_with(['.', '~']);
        if is_command_position(before_cursor, command_start) && !looks_like_a_path {
            let mut candidates = command_completions(&self.command_names, current_word);
            // Without a prefix every program of `PATH` would be listed.
            if !current_word.is_empty() {
                for program in self.programs_starting_with(current_word) {
                    if !self.command_names.contains(&program) {
                        candidates.push(Pair {
                            replacement: format!("{program} "),
                            display: program,
                        });
                    }
                }
            }
            candidates.sort_by(|left, right| left.display.cmp(&right.display));
            return Ok((command_start, candidates));
        }

        self.filename_completer.complete(line, pos, context)
    }
}

/// The largest index not after `pos` that falls between two characters.
fn floor_char_boundary(text: &str, pos: usize) -> usize {
    let mut index = pos.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

/// Byte index where the last word of `text` starts. A whitespace character can
/// take several bytes (a no-break space takes two), so its width is used rather
/// than assuming one.
fn word_start(text: &str) -> usize {
    text.char_indices()
        .rev()
        .find(|(_, character)| character.is_whitespace())
        .map_or(0, |(index, character)| index + character.len_utf8())
}

fn command_completions(command_names: &[String], prefix: &str) -> Vec<Pair> {
    command_names
        .iter()
        .filter(|command| command.starts_with(prefix))
        .map(|command| Pair {
            display: command.clone(),
            replacement: format!("{command} "),
        })
        .collect()
}

impl Hinter for ShellHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, context: &Context<'_>) -> Option<Self::Hint> {
        self.history_hinter
            .hint(line, pos, context)
            .filter(|hint| !hint.chars().any(char::is_control))
    }
}

impl Highlighter for ShellHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\u{1b}[2;90m{hint}\u{1b}[0m"))
    }
}

impl Validator for ShellHelper {}

impl Helper for ShellHelper {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command;
    use rustyline::Config;
    use rustyline::history::{DefaultHistory, History};
    use std::fs;
    use std::path::MAIN_SEPARATOR;
    use std::time::{SystemTime, UNIX_EPOCH};

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
    fn a_word_starts_after_a_multi_byte_whitespace() {
        // U+00A0 (no-break space) and U+3000 (ideographic space) take 2 and 3 bytes.
        assert_eq!(word_start("ls\u{a0}"), "ls\u{a0}".len());
        assert_eq!(word_start("ls\u{3000}fi"), "ls\u{3000}".len());
        assert_eq!(word_start("ls  fi"), 4);
        assert_eq!(word_start("ls"), 0);
        assert_eq!(word_start(""), 0);
        assert_eq!(word_start("\u{e9}cho x"), "\u{e9}cho ".len());
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
        let helper = ShellHelper::with_command_names(vec!["pwd".into(), "ls".into()])
            .without_path_programs();

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
        let helper =
            ShellHelper::with_command_names(commands.iter().map(|c| c.to_string()).collect());
        *helper.programs.borrow_mut() = Some(ProgramCache {
            path: shell_env::value("PATH")
                .map(OsString::from)
                .unwrap_or_default(),
            loaded: Instant::now(),
            names: programs.iter().map(|p| p.to_string()).collect(),
        });
        helper
    }

    #[test]
    fn a_command_position_follows_every_separator() {
        assert!(is_command_position("", 0));
        assert!(is_command_position("   ", 3));
        assert!(is_command_position("ls | ", 5));
        assert!(is_command_position("a;", 2));
        assert!(is_command_position("a && ", 5));
        assert!(is_command_position("a || ", 5));
        assert!(is_command_position("a|", 2));
        assert!(!is_command_position("ls ", 3));
        assert!(!is_command_position("echo a | cat x ", 15));
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
}
