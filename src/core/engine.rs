use std::env;
use std::path::PathBuf;

use crate::app::command::{self, CommandResult, ShellState};
use crate::app::parser::parse_input;
use crate::commands::user::UserCommandRegistry;
use crate::config::AppConfig;
use crate::render;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExecutionStatus {
    Continue,
    Exit,
    Failed,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DirectoryChange {
    pub previous: PathBuf,
    pub current: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub stdout: String,
    pub stderr: String,
    pub error: Option<String>,
    pub current_dir: Option<PathBuf>,
    pub directory_change: Option<DirectoryChange>,
}

impl ExecutionResult {
    pub fn succeeded(&self) -> bool {
        self.status != ExecutionStatus::Failed
    }
}

pub struct ShellEngine {
    state: ShellState,
}

impl ShellEngine {
    pub fn new() -> Self {
        Self {
            state: ShellState::new(UserCommandRegistry::default()),
        }
    }

    pub(crate) fn from_config(config: &AppConfig) -> Result<Self, String> {
        let user_commands = UserCommandRegistry::new(&config.commands, command::command_names())?;
        Ok(Self {
            state: ShellState::new(user_commands),
        })
    }

    pub(crate) fn command_names(&self) -> Vec<String> {
        command::command_names()
            .map(str::to_string)
            .chain(self.state.user_command_names().map(str::to_string))
            .collect()
    }

    pub fn execute_line(&mut self, input: &str) -> ExecutionResult {
        match parse_input(input) {
            Ok(args) if args.is_empty() => self.empty_result(),
            Ok(args) => self.execute_args(&args),
            Err(error) => self.failed_result(error),
        }
    }

    pub fn execute_args(&mut self, args: &[String]) -> ExecutionResult {
        let previous_dir = env::current_dir().ok();
        let (command_result, output) =
            render::capture_output(|| command::execute(args, &mut self.state));
        let current_dir = env::current_dir().ok();
        let directory_change = match (&previous_dir, &current_dir) {
            (Some(previous), Some(current)) if previous != current => Some(DirectoryChange {
                previous: previous.clone(),
                current: current.clone(),
            }),
            _ => None,
        };

        let (status, error) = match command_result {
            CommandResult::Continue => (ExecutionStatus::Continue, None),
            CommandResult::Exit => (ExecutionStatus::Exit, None),
            CommandResult::Error(error) => (ExecutionStatus::Failed, Some(error)),
        };

        ExecutionResult {
            status,
            stdout: output.stdout,
            stderr: output.stderr,
            error,
            current_dir,
            directory_change,
        }
    }

    fn empty_result(&self) -> ExecutionResult {
        ExecutionResult {
            status: ExecutionStatus::Continue,
            stdout: String::new(),
            stderr: String::new(),
            error: None,
            current_dir: env::current_dir().ok(),
            directory_change: None,
        }
    }

    fn failed_result(&self, error: String) -> ExecutionResult {
        ExecutionResult {
            status: ExecutionStatus::Failed,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(error),
            current_dir: env::current_dir().ok(),
            directory_change: None,
        }
    }
}

impl Default for ShellEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use crate::test_support::cwd_lock;
    use crate::utils::path::temp_path;

    #[test]
    fn captures_command_output_without_writing_to_the_terminal() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let current_dir = env::current_dir().expect("current directory should be readable");
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("pwd");

        assert_eq!(result.status, ExecutionStatus::Continue);
        assert_eq!(result.stdout.trim(), current_dir.display().to_string());
        assert!(result.stderr.is_empty());
        assert!(result.error.is_none());
    }

    #[test]
    fn reports_parse_errors_as_typed_failures() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("cd \"unfinished");

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(result.error.as_deref(), Some("missing closing quote"));
        assert!(result.directory_change.is_none());
    }

    #[test]
    fn exposes_directory_changes() {
        let _guard = cwd_lock().lock().expect("cwd lock is poisoned");
        let original_dir = env::current_dir().expect("current directory should be readable");
        let target_dir = temp_path("engine", "directory");
        fs::create_dir(&target_dir).expect("test directory should be created");
        let mut engine = ShellEngine::new();

        let result =
            engine.execute_args(&["cd".to_string(), target_dir.to_string_lossy().into_owned()]);

        assert_eq!(result.status, ExecutionStatus::Continue);
        assert_eq!(
            result.directory_change,
            Some(DirectoryChange {
                previous: original_dir.clone(),
                current: target_dir.clone(),
            })
        );
        assert_eq!(result.current_dir.as_deref(), Some(target_dir.as_path()));

        env::set_current_dir(&original_dir).expect("test should restore current directory");
        fs::remove_dir(target_dir).expect("test directory should be removed");
    }

    #[test]
    fn expands_builtin_custom_commands_and_appends_arguments() {
        let mut config = AppConfig::default();
        config.commands.insert(
            "show-theme".into(),
            crate::config::UserCommandConfig {
                kind: crate::config::UserCommandKind::Builtin,
                target: "theme".into(),
                args: vec!["preview".into()],
                description: "Preview a theme".into(),
                ..Default::default()
            },
        );
        let mut engine = ShellEngine::from_config(&config).unwrap();

        let result = engine.execute_line("show-theme minimal");

        assert_eq!(result.status, ExecutionStatus::Continue);
        assert!(result.stdout.contains("Theme: minimal"));
        assert!(engine.command_names().contains(&"show-theme".into()));
    }
}
