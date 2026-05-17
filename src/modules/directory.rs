use std::env;

fn internal_format(path: &str) -> String {
    format!("{}", path)
}

pub fn pwd() -> std::io::Result<()> {
    let path = env::current_dir()?;
    let formatted = internal_format(
        path.to_str().unwrap()
    );
    println!("{}", formatted);
    Ok(())
}