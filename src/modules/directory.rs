use std::env;

fn internal_format(path: &std::path::Path) -> String {
    path.display().to_string()
}

pub fn pwd() -> std::io::Result<()> {
    let path = env::current_dir()?;
    let formatted = internal_format(&path);
    println!("{}", formatted);
    Ok(())
}
