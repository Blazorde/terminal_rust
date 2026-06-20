use std::env;
use std::path::PathBuf;

pub fn trimmed_arg(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub fn required_path<E>(value: Option<&str>, error: E) -> Result<PathBuf, E> {
    trimmed_arg(value).map(PathBuf::from).ok_or(error)
}

pub fn home_directory() -> Option<PathBuf> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(PathBuf::from))
}

pub fn expand_home(path: &str) -> Option<PathBuf> {
    match path {
        "~" => home_directory(),
        value if value.starts_with("~/") || value.starts_with("~\\") => {
            home_directory().map(|home| home.join(&value[2..]))
        }
        value => Some(PathBuf::from(value)),
    }
}

#[cfg(test)]
pub fn temp_path(prefix: &str, name: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after Unix epoch")
        .as_nanos();

    env::temp_dir().join(format!("terminal-rust-{prefix}-{name}-{unique}"))
}
