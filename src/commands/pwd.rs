use std::env;

use crate::render;

pub fn pwd() -> std::io::Result<()> {
    let path = env::current_dir()?;
    render::stdout_line(path.display())
}
