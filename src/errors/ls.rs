use std::fmt;
use std::io;
use std::path::PathBuf;

#[derive(Debug)]
pub enum LsError {
    DirectoryEntry { path: PathBuf, source: io::Error },
    Metadata { path: PathBuf, source: io::Error },
    Output(io::Error),
    ReadDirectory { path: PathBuf, source: io::Error },
    UnknownFlag(char),
    UnknownOption(String),
    UnknownSortMode(String),
}

impl fmt::Display for LsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DirectoryEntry { path, source } => {
                write!(
                    formatter,
                    "cannot read an entry in '{}': {source}",
                    path.display()
                )
            }
            Self::Metadata { path, source } => {
                write!(
                    formatter,
                    "cannot read metadata for '{}': {source}",
                    path.display()
                )
            }
            Self::Output(source) => write!(formatter, "cannot write output: {source}"),
            Self::ReadDirectory { path, source } => {
                write!(
                    formatter,
                    "cannot read directory '{}': {source}",
                    path.display()
                )
            }
            Self::UnknownFlag(flag) => write!(formatter, "unknown flag '-{flag}'"),
            Self::UnknownOption(option) => write!(formatter, "unknown option '{option}'"),
            Self::UnknownSortMode(sort_mode) => {
                write!(formatter, "unknown sort mode '{sort_mode}'")
            }
        }
    }
}

impl std::error::Error for LsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DirectoryEntry { source, .. }
            | Self::Metadata { source, .. }
            | Self::ReadDirectory { source, .. }
            | Self::Output(source) => Some(source),
            Self::UnknownFlag(_) | Self::UnknownOption(_) | Self::UnknownSortMode(_) => None,
        }
    }
}
