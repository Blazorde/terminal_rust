mod app;
mod cache;
mod commands;
mod config;
mod errors;
pub mod git;
mod prompt;
mod render;
mod theme;
mod utils;

use std::process::ExitCode;

fn main() -> ExitCode {
    app::run()
}
