use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

pub struct ShellHelper {
    filename_completer: FilenameCompleter,
    command_names: Vec<String>,
}

impl ShellHelper {
    pub fn with_command_names(command_names: Vec<String>) -> Self {
        Self {
            filename_completer: FilenameCompleter::new(),
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
}

impl Highlighter for ShellHelper {}

impl Validator for ShellHelper {}

impl Helper for ShellHelper {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::command;

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
    fn plugins_is_available_in_completion() {
        let completions = completions("plug");

        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].replacement, "plugins ");
    }

    #[test]
    fn custom_commands_are_available_in_completion() {
        let names = vec!["pwd".into(), "serve".into()];
        let completions = command_completions(&names, "ser");

        assert_eq!(completions[0].replacement, "serve ");
    }
}
