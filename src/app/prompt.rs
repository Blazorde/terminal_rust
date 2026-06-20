use std::env;
use std::io;

use crate::render;

pub fn prompt() -> io::Result<String> {
    let current_dir = env::current_dir()?;
    Ok(format!("{}> ", current_dir.display()))
}

pub fn print_prompt() -> io::Result<()> {
    render::stdout(prompt()?)
}
