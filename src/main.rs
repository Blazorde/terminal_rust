mod app;
mod cache;
mod commands;
mod errors;
pub mod git;
mod prompt;
mod render;
mod utils;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::run()
}
