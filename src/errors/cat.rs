use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum CatError {
    HomeDirectoryNotFound,
    MissingFileName,
    NotAFile(PathBuf),
    NotFound(PathBuf),
    ReadFailed { path: PathBuf, source: io::Error },
}

impl fmt::Display for CatError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HomeDirectoryNotFound => {
                write!(formatter, "cat: could not find the home directory")
            }
            Self::MissingFileName => write!(formatter, "cat: missing file name"),
            Self::NotAFile(path) => write!(formatter, "cat: '{}' is not a file", path.display()),
            Self::NotFound(path) => write!(
                formatter,
                "cat: '{}': no such file or directory",
                path.display()
            ),
            Self::ReadFailed { path, source } => {
                write!(formatter, "cat: cannot read '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for CatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadFailed { source, .. } => Some(source),
            Self::HomeDirectoryNotFound
            | Self::MissingFileName
            | Self::NotAFile(_)
            | Self::NotFound(_) => None,
        }
    }
}
