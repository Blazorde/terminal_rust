use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CdError {
    CurrentDirectoryUnavailable(io::Error),
    HomeDirectoryNotFound,
    NotADirectory(PathBuf),
    PreviousDirectoryNotFound,
    ChangeDirectoryFailed { path: PathBuf, source: io::Error },
}

impl fmt::Display for CdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirectoryUnavailable(source) => {
                write!(formatter, "cd: cannot read current directory: {source}")
            }
            Self::HomeDirectoryNotFound => {
                write!(formatter, "cd: could not find the home directory")
            }
            Self::NotADirectory(path) => {
                write!(formatter, "cd: '{}' is not a directory", path.display())
            }
            Self::PreviousDirectoryNotFound => {
                write!(formatter, "cd: previous directory is not available")
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
            Self::CurrentDirectoryUnavailable(source) => Some(source),
            Self::ChangeDirectoryFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}
