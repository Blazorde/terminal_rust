mod app;
mod commands;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::run()
}
