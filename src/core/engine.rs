use std::env;
use std::path::PathBuf;

use crate::app::command::{self, CommandResult, ShellState};
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

    /// The code a front end should exit with: the one given to `exit`, or the
    /// status of the last command.
    pub fn exit_code(&self) -> u8 {
        self.state.exit_code()
    }

    pub(crate) fn startup_script(&self) -> Option<&str> {
        self.state.startup_script()
    }

    pub fn execute_line(&mut self, input: &str) -> ExecutionResult {
        if input.trim().is_empty() {
            return self.empty_result();
        }
        self.run_captured(|state| command::run_line(input, state))
    }

    pub fn execute_args(&mut self, args: &[String]) -> ExecutionResult {
        self.run_captured(|state| {
            let result = command::execute(args, state);
            state.record_status(matches!(result, CommandResult::Error(_)));
            result
        })
    }

    fn run_captured(
        &mut self,
        run: impl FnOnce(&mut ShellState) -> CommandResult,
    ) -> ExecutionResult {
        let previous_dir = env::current_dir().ok();
        let (command_result, output) = render::capture_output(|| run(&mut self.state));
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
            // An empty message means the error was already written, to a redirection.
            CommandResult::Error(error) => (
                ExecutionStatus::Failed,
                Some(error).filter(|e| !e.is_empty()),
            ),
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

    #[test]
    fn a_variable_set_on_one_line_is_expanded_on_the_next() {
        let mut engine = ShellEngine::new();

        engine.execute_line("TRS_ENGINE_SET=five");
        let result = engine.execute_line("echo $TRS_ENGINE_SET and ${TRS_ENGINE_SET}");

        assert_eq!(result.stdout, "five and five\n");
    }

    #[test]
    fn single_quotes_keep_the_dollar_sign() {
        let mut engine = ShellEngine::new();
        engine.execute_line("TRS_ENGINE_QUOTE=x");

        let result = engine.execute_line("echo '$TRS_ENGINE_QUOTE' \"$TRS_ENGINE_QUOTE\"");

        assert_eq!(result.stdout, "$TRS_ENGINE_QUOTE x\n");
    }

    #[test]
    fn the_last_status_follows_success_and_failure() {
        let mut engine = ShellEngine::new();

        engine.execute_line("echo fine");
        assert_eq!(engine.execute_line("echo $?").stdout, "0\n");

        // A program that does not exist is 127, as in POSIX shells.
        engine.execute_line("terminal-rust-no-such-program");
        assert_eq!(engine.execute_line("echo $?").stdout, "127\n");

        // A command that fails inside the shell is 1.
        engine.execute_line("cat");
        assert_eq!(engine.execute_line("echo $?").stdout, "1\n");

        // A line that cannot be parsed fails too.
        engine.execute_line("echo fine");
        engine.execute_line("echo 'unclosed");
        assert_eq!(engine.execute_line("echo $?").stdout, "1\n");
    }

    #[test]
    fn export_makes_a_variable_visible_to_printenv() {
        let mut engine = ShellEngine::new();

        engine.execute_line("export TRS_ENGINE_EXPORT=shared");
        let result = engine.execute_line("printenv TRS_ENGINE_EXPORT");

        assert_eq!(result.stdout, "shared\n");
    }

    #[test]
    fn an_unexported_variable_is_not_part_of_the_environment() {
        let mut engine = ShellEngine::new();

        engine.execute_line("TRS_ENGINE_LOCAL=private");
        let result = engine.execute_line("printenv TRS_ENGINE_LOCAL");

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(result.stdout, "");
    }

    #[test]
    fn export_without_a_value_exports_the_current_one() {
        let mut engine = ShellEngine::new();
        engine.execute_line("TRS_ENGINE_PROMOTE=kept");

        engine.execute_line("export TRS_ENGINE_PROMOTE");

        assert_eq!(
            engine.execute_line("printenv TRS_ENGINE_PROMOTE").stdout,
            "kept\n"
        );
    }

    #[test]
    fn assigning_an_exported_variable_updates_what_programs_receive() {
        let mut engine = ShellEngine::new();
        engine.execute_line("export TRS_ENGINE_UPDATE=old");

        engine.execute_line("TRS_ENGINE_UPDATE=new");

        assert_eq!(
            engine.execute_line("printenv TRS_ENGINE_UPDATE").stdout,
            "new\n"
        );
    }

    #[test]
    fn unset_removes_local_and_exported_variables() {
        let mut engine = ShellEngine::new();
        engine.execute_line("export TRS_ENGINE_GONE=1");
        engine.execute_line("TRS_ENGINE_GONE_LOCAL=2");

        engine.execute_line("unset TRS_ENGINE_GONE TRS_ENGINE_GONE_LOCAL");

        assert_eq!(
            engine
                .execute_line("echo [$TRS_ENGINE_GONE][$TRS_ENGINE_GONE_LOCAL]")
                .stdout,
            "[][]\n"
        );
        assert_eq!(
            engine.execute_line("printenv TRS_ENGINE_GONE").status,
            ExecutionStatus::Failed
        );
    }

    #[test]
    fn export_and_unset_reject_invalid_names() {
        let mut engine = ShellEngine::new();

        let export = engine.execute_line("export 1bad=x");
        let unset = engine.execute_line("unset bad-name");
        let missing = engine.execute_line("unset");

        assert_eq!(
            export.error.as_deref(),
            Some("export: not a valid name: 1bad")
        );
        assert_eq!(
            unset.error.as_deref(),
            Some("unset: not a valid name: bad-name")
        );
        assert_eq!(missing.status, ExecutionStatus::Failed);
    }

    #[test]
    fn a_word_that_is_not_an_assignment_is_still_a_command() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("1bad=x");

        assert_eq!(result.error.as_deref(), Some("unknown command: 1bad=x"));
    }

    #[test]
    fn a_semicolon_runs_both_commands_in_order() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo one ; echo two");

        assert_eq!(result.stdout, "one\ntwo\n");
        assert_eq!(result.status, ExecutionStatus::Continue);
    }

    #[test]
    fn and_skips_the_next_command_after_a_failure() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("cd /terminal-rust/no/such/directory && echo skipped");

        assert_eq!(result.stdout, "");
        assert_eq!(result.status, ExecutionStatus::Failed);
        assert!(result.error.is_some());
    }

    #[test]
    fn and_runs_the_next_command_after_a_success() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo first && echo second");

        assert_eq!(result.stdout, "first\nsecond\n");
    }

    #[test]
    fn or_runs_the_next_command_only_after_a_failure() {
        let mut engine = ShellEngine::new();

        let recovered =
            engine.execute_line("cd /terminal-rust/no/such/directory || echo recovered");
        let skipped = engine.execute_line("echo fine || echo never");

        assert_eq!(recovered.stdout, "recovered\n");
        assert_eq!(recovered.status, ExecutionStatus::Continue);
        assert!(
            recovered.stderr.contains("cd:"),
            "the skipped failure should still be reported: {}",
            recovered.stderr
        );
        assert_eq!(skipped.stdout, "fine\n");
    }

    #[test]
    fn the_status_variable_follows_each_segment_of_the_line() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("cd /terminal-rust/no/such/directory || echo status $?");

        assert_eq!(result.stdout, "status 1\n");
    }

    #[test]
    fn a_variable_set_in_one_segment_is_visible_in_the_next() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("TRS_CHAIN_VALUE=carried ; echo $TRS_CHAIN_VALUE");

        assert_eq!(result.stdout, "carried\n");
    }

    #[test]
    fn exit_stops_the_rest_of_the_line() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo before ; exit ; echo after");

        assert_eq!(result.status, ExecutionStatus::Exit);
        assert_eq!(result.stdout, "before\n");
    }

    #[test]
    fn the_last_segment_decides_the_final_status() {
        let mut engine = ShellEngine::new();

        let failed_last = engine.execute_line("echo ok ; cd /terminal-rust/no/such/directory");
        let succeeded_last = engine.execute_line("cd /terminal-rust/no/such/directory ; echo ok");

        assert_eq!(failed_last.status, ExecutionStatus::Failed);
        assert_eq!(succeeded_last.status, ExecutionStatus::Continue);
    }

    #[test]
    fn a_built_in_pipes_its_output_to_the_next_built_in() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo hello | cat");

        assert_eq!(result.status, ExecutionStatus::Continue);
        assert_eq!(result.stdout, "hello\n");
        assert_eq!(result.error, None);
    }

    #[test]
    fn a_pipeline_with_a_missing_command_is_rejected_before_anything_runs() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo first ; echo hello |");

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(
            result.error.as_deref(),
            Some("expected a command after '|'")
        );
        assert_eq!(result.stdout, "");
    }

    #[test]
    fn an_error_written_to_a_redirection_is_not_printed_again() {
        let directory = temp_path("engine", "redirected-error");
        fs::create_dir(&directory).expect("directory should be created");
        let log = directory.join("err.txt");
        let mut engine = ShellEngine::new();

        let result = engine.execute_line(&format!(
            "cat \"{}\" 2> \"{}\"",
            directory.join("missing.txt").display(),
            log.display()
        ));

        assert_eq!(result.status, ExecutionStatus::Failed);
        assert_eq!(result.error, None);
        assert!(
            fs::read_to_string(&log)
                .expect("the log should exist")
                .contains("missing.txt")
        );
        fs::remove_dir_all(directory).expect("directory should be removed");
    }

    #[test]
    fn nothing_runs_when_a_later_segment_has_a_syntax_error() {
        let mut engine = ShellEngine::new();

        let result = engine.execute_line("echo first ; && echo second");

        assert_eq!(result.stdout, "");
        assert!(result.error.is_some());
    }
}
