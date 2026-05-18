use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum CommandError {
    TooManyArguments { command: &'static str },
    UnknownCommand(String),
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyArguments { command } => {
                write!(formatter, "{command}: too many arguments")
            }
            Self::UnknownCommand(command) => write!(formatter, "unknown command: {command}"),
        }
    }
}

impl std::error::Error for CommandError {}
