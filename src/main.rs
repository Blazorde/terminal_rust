mod app;
mod commands;
mod errors;
mod render;
mod utils;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::run()
}
