use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum MkdirError {
    AlreadyExists(PathBuf),
    CreateFailed { path: PathBuf, source: io::Error },
    MissingDirectoryName,
}

impl fmt::Display for MkdirError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(path) => {
                write!(formatter, "mkdir: '{}' already exists", path.display())
            }
            Self::CreateFailed { path, source } => {
                write!(
                    formatter,
                    "mkdir: cannot create '{}': {source}",
                    path.display()
                )
            }
            Self::MissingDirectoryName => write!(formatter, "mkdir: missing directory name"),
        }
    }
}

impl std::error::Error for MkdirError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateFailed { source, .. } => Some(source),
            Self::AlreadyExists(_) | Self::MissingDirectoryName => None,
        }
    }
}
