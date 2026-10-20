mod app;
mod launch;
mod pty;

use std::process::ExitCode;

type NativeResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type TerminalStream = (egui_tty::TtyStream, std::sync::mpsc::Sender<Vec<u8>>);

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.as_slice() == ["--pty-shell"] {
        return terminal_rust::app::run_interactive();
    }
    if matches!(arguments.as_slice(), [flag] if flag == "--help" || flag == "-h") {
        println!("{}", launch::USAGE);
        return ExitCode::SUCCESS;
    }

    let overrides = match launch::parse_options(&arguments) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    match app::run(overrides) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("cannot start native Terminal Rust prototype: {error}");
            ExitCode::FAILURE
        }
    }
}
