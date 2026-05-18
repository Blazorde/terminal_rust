mod app;
mod commands;
mod errors;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::run()
}
