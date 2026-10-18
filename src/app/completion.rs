use std::borrow::Cow;

use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::{Hinter, HistoryHinter};
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct ShellHelper {
    filename_completer: FilenameCompleter,
    history_hinter: HistoryHinter,
    command_names: Vec<String>,
}

impl ShellHelper {
    pub fn with_command_names(command_names: Vec<String>) -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
            history_hinter: HistoryHinter::new(),
            command_names,
        }
    }
}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let safe_pos = pos.min(line.len());
        let before_cursor = &line[..safe_pos];
        let command_start = before_cursor
            .rfind(char::is_whitespace)
            .map_or(0, |index| index + 1);
        let current_word = &before_cursor[command_start..];

        if command_start == 0 {
            return Ok((0, command_completions(&self.command_names, current_word)));
        }

        self.filename_completer.complete(line, pos, context)
    }
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
}
