use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum RmError {
    DeleteFailed { path: PathBuf, source: io::Error },
    InvalidOption(String),
    IsDirectory(PathBuf),
    MissingTarget,
    NotFound(PathBuf),
}

impl fmt::Display for RmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeleteFailed { path, source } => {
                write!(
                    formatter,
                    "rm: cannot remove '{}': {source}",
                    path.display()
                )
            }
            Self::InvalidOption(option) => write!(formatter, "rm: invalid option '{option}'"),
            Self::IsDirectory(path) => write!(
                formatter,
                "rm: cannot remove '{}': is a directory",
                path.display()
            ),
            Self::MissingTarget => write!(formatter, "rm: missing target"),
            Self::NotFound(path) => write!(
                formatter,
                "rm: cannot remove '{}': no such file or directory",
                path.display()
            ),
        }
    }
}

impl std::error::Error for RmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DeleteFailed { source, .. } => Some(source),
            Self::InvalidOption(_)
            | Self::IsDirectory(_)
            | Self::MissingTarget
            | Self::NotFound(_) => None,
        }
    }
}
