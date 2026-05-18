use std::env;
use std::io::{self, Write};

pub fn prompt() -> io::Result<String> {
    let current_dir = env::current_dir()?;
    Ok(format!("{}> ", current_dir.display()))
}

pub fn print_prompt() -> io::Result<()> {
    print!("{}", prompt()?);
    io::stdout().flush()
}
