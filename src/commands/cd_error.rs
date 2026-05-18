use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CdError {
    HomeDirectoryNotFound,
    NotADirectory(PathBuf),
    ChangeDirectoryFailed { path: PathBuf, source: io::Error },
}

impl fmt::Display for CdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeDirectoryNotFound => {
                write!(formatter, "cd: could not find the home directory")
            }
            Self::NotADirectory(path) => {
                write!(formatter, "cd: '{}' is not a directory", path.display())
            }
            Self::ChangeDirectoryFailed { path, source } => {
                write!(
                    formatter,
                    "cd: cannot change directory to '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for CdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ChangeDirectoryFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}
