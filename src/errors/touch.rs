use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum TouchError {
    CreateFailed { path: PathBuf, source: io::Error },
    MissingFileName,
    TargetIsDirectory(PathBuf),
}

impl fmt::Display for TouchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFailed { path, source } => {
                write!(
                    formatter,
                    "touch: cannot create '{}': {source}",
                    path.display()
                )
            }
            Self::MissingFileName => write!(formatter, "touch: missing file name"),
            Self::TargetIsDirectory(path) => {
                write!(formatter, "touch: '{}' is a directory", path.display())
            }
        }
    }
}

impl std::error::Error for TouchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CreateFailed { source, .. } => Some(source),
            Self::MissingFileName | Self::TargetIsDirectory(_) => None,
        }
    }
}
