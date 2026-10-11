use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--pty-shell") {
        terminal_rust::app::run_interactive()
    } else {
        terminal_rust_desktop_lib::run();
        ExitCode::SUCCESS
    }
}
