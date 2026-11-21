use std::env;

use crate::render;
use crate::render::printable::printable;

pub fn pwd() -> std::io::Result<()> {
    let path = env::current_dir()?;
    render::stdout_line(printable(&path.display().to_string()))
}
