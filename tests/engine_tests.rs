use terminal_rust::{ExecutionStatus, ShellEngine};

#[test]
fn public_engine_executes_the_same_help_command_as_the_cli() {
    let mut engine = ShellEngine::new();

    let result = engine.execute_line("help");

    assert_eq!(result.status, ExecutionStatus::Continue);
    assert!(result.stdout.contains("Available commands:"));
    assert!(result.stdout.contains("dashboard"));
    assert!(result.stderr.is_empty());
    assert!(result.error.is_none());
}

#[test]
fn public_engine_exposes_command_errors_without_printing_them() {
    let mut engine = ShellEngine::new();

    let result = engine.execute_line("not-a-command");

    assert_eq!(result.status, ExecutionStatus::Failed);
    assert_eq!(
        result.error.as_deref(),
        Some("unknown command: not-a-command")
    );
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
}
