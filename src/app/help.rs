use std::io;

use crate::render;

pub fn print_help<'a>(commands: impl Iterator<Item = (&'a str, &'a str)>) -> io::Result<()> {
    let mut output = String::from("Available commands:\n");
    for (usage, summary) in commands {
        output.push_str(&format!("  {usage:<28} {summary}\n"));
    }
    render::stdout(output)
}
