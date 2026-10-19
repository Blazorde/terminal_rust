use std::env;
use std::path::PathBuf;

use crate::app::command::{self, CommandResult, ShellState};
use crate::app::parser::parse_input;
use crate::commands::user::UserCommandRegistry;
use crate::config::AppConfig;
use crate::plugin::DeclarativeCommandRegistry;
use crate::plugin::registry;
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
            state: ShellState::new(
                UserCommandRegistry::default(),
                DeclarativeCommandRegistry::default(),
                None,
            ),
        }
    }

    pub(crate) fn from_config(config: &AppConfig) -> Result<Self, String> {
        let entries = registry::discover_default(&config.plugins.enabled)
            .map_err(|error| format!("cannot discover plugins: {error}"))?;
        Self::from_config_and_entries(config, &entries)
    }

    fn from_config_and_entries(
        config: &AppConfig,
        entries: &[registry::RegistryEntry],
    ) -> Result<Self, String> {
        let user_commands =
            UserCommandRegistry::new(&config.commands, &config.aliases, command::command_names())?;
        let reserved_names = command::command_names()
            .map(str::to_string)
            .chain(user_commands.names());
        let plugin_commands = DeclarativeCommandRegistry::from_entries(
            entries,
            &config.plugins.grants,
            reserved_names,
        );
        Ok(Self {
            state: ShellState::new(
                user_commands,
                plugin_commands,
                config.startup_script.clone(),
            ),
        })
    }

    pub(crate) fn command_names(&self) -> Vec<String> {
        command::command_names()
            .map(str::to_string)
            .chain(self.state.user_command_names())
            .chain(self.state.plugin_command_names().map(str::to_string))
            .collect()
    }

    pub(crate) fn startup_script(&self) -> Option<&str> {
        self.state.startup_script()
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

    fn plugin_fixture(root: &std::path::Path) {
        let directory = root.join("example.hello").join("0.1.0");
        fs::create_dir_all(&directory).expect("plugin fixture directory should be created");
        fs::write(
            directory.join("terminal-rust-plugin.toml"),
            r#"schema_version = 1
id = "example.hello"
name = "Hello plugin"
version = "0.1.0"
description = "Official declarative fixture"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
permissions = ["command.register", "output.write"]

[[commands]]
name = "hello"
summary = "Print a greeting"
response = "Hello from the official Terminal Rust plugin!\n"
"#,
        )
        .expect("plugin fixture manifest should be written");
    }

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
    fn cat_and_rm_accept_quoted_paths_with_spaces() {
        let root = temp_path("engine", "quoted paths");
        fs::create_dir(&root).expect("quoted-path fixture directory should be created");
        let double_quoted_file = root.join("file with spaces.txt");
        let single_quoted_file = root.join("another file.txt");
        fs::write(&double_quoted_file, "double quoted content")
            .expect("double-quoted fixture should be written");
        fs::write(&single_quoted_file, "single quoted content")
            .expect("single-quoted fixture should be written");

        let mut engine = ShellEngine::new();
        let cat_double = engine.execute_line(&format!("cat \"{}\"", double_quoted_file.display()));
        assert_eq!(cat_double.status, ExecutionStatus::Continue);
        assert_eq!(cat_double.stdout, "double quoted content");

        let cat_single = engine.execute_line(&format!("cat '{}'", single_quoted_file.display()));
        assert_eq!(cat_single.status, ExecutionStatus::Continue);
        assert_eq!(cat_single.stdout, "single quoted content");

        let rm_single = engine.execute_line(&format!("rm '{}'", single_quoted_file.display()));
        assert_eq!(rm_single.status, ExecutionStatus::Continue);
        assert!(!single_quoted_file.exists());

        let empty_directory = root.join("empty directory");
        fs::create_dir(&empty_directory).expect("empty directory fixture should be created");
        let rm_double = engine.execute_line(&format!("rm -d \"{}\"", empty_directory.display()));
        assert_eq!(rm_double.status, ExecutionStatus::Continue);
        assert!(!empty_directory.exists());

        fs::remove_dir_all(root).expect("quoted-path fixtures should be removed");
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
                current: result
                    .current_dir
                    .clone()
                    .expect("current directory should be available"),
            })
        );
        assert_eq!(
            fs::canonicalize(
                result
                    .current_dir
                    .as_deref()
                    .expect("current directory should be available")
            )
            .expect("current directory should resolve"),
            fs::canonicalize(&target_dir).expect("target directory should resolve")
        );

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

    #[test]
    fn executes_and_disables_an_authorized_declarative_plugin_command() {
        let root = temp_path("engine", "declarative-plugin");
        fs::create_dir(&root).expect("plugin root should be created");
        plugin_fixture(&root);

        let mut config = AppConfig::default();
        config.plugins.enabled.push("example.hello".into());
        config.plugins.grants.insert(
            "example.hello".into(),
            vec!["command.register".into(), "output.write".into()],
        );
        let entries = registry::discover(&root, &config.plugins.enabled).unwrap();
        let mut engine = ShellEngine::from_config_and_entries(&config, &entries).unwrap();

        let result = engine.execute_line("hello");

        assert_eq!(result.status, ExecutionStatus::Continue);
        assert_eq!(
            result.stdout,
            "Hello from the official Terminal Rust plugin!\n"
        );
        assert!(engine.command_names().contains(&"hello".into()));

        config.plugins.enabled.clear();
        let entries = registry::discover(&root, &config.plugins.enabled).unwrap();
        let mut disabled_engine = ShellEngine::from_config_and_entries(&config, &entries).unwrap();
        let disabled = disabled_engine.execute_line("hello");
        let native = disabled_engine.execute_line("pwd");

        assert_eq!(disabled.status, ExecutionStatus::Failed);
        assert_eq!(disabled.error.as_deref(), Some("unknown command: hello"));
        assert_eq!(native.status, ExecutionStatus::Continue);

        fs::remove_dir_all(root).expect("plugin fixture should be removed");
    }

    #[test]
    fn keeps_a_plugin_command_hidden_without_permission_grants() {
        let root = temp_path("engine", "ungranted-plugin");
        fs::create_dir(&root).expect("plugin root should be created");
        plugin_fixture(&root);

        let mut config = AppConfig::default();
        config.plugins.enabled.push("example.hello".into());
        let entries = registry::discover(&root, &config.plugins.enabled).unwrap();
        let mut engine = ShellEngine::from_config_and_entries(&config, &entries).unwrap();

        let result = engine.execute_line("hello");

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(
            result.error.as_deref(),
            Some(
                "plugin 'example.hello' cannot use permission 'command.register': has not been granted by the user"
            )
        );
        fs::remove_dir_all(root).expect("plugin fixture should be removed");
    }
}
