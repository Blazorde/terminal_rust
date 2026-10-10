use std::error::Error;
use std::fmt;
use std::io::{self, IsTerminal};

use crate::render;
use crate::theme::{self, ThemeError};

#[derive(Debug)]
pub enum ThemeCommandError {
    Usage,
    UnknownAction(String),
    MissingTheme,
    TooManyArguments,
    Theme(ThemeError),
    Output(io::Error),
}

impl fmt::Display for ThemeCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(formatter, "theme: expected 'list' or 'preview <name>'"),
            Self::UnknownAction(action) => write!(
                formatter,
                "theme: unknown action '{action}' (expected 'list' or 'preview')"
            ),
            Self::MissingTheme => write!(formatter, "theme: missing theme name after 'preview'"),
            Self::TooManyArguments => write!(formatter, "theme: too many arguments"),
            Self::Theme(error) => write!(formatter, "theme: {error}"),
            Self::Output(error) => write!(formatter, "theme: cannot write output: {error}"),
        }
    }
}

impl Error for ThemeCommandError {}

pub fn theme(args: &[String]) -> Result<(), ThemeCommandError> {
    match args.first().map(String::as_str) {
        None => Err(ThemeCommandError::Usage),
        Some("list") if args.len() == 1 => list(),
        Some("list") => Err(ThemeCommandError::TooManyArguments),
        Some("preview") => preview(&args[1..]),
        Some(action) => Err(ThemeCommandError::UnknownAction(action.to_string())),
    }
}

fn list() -> Result<(), ThemeCommandError> {
    let mut output = String::from("Available themes:\n");
    for preset in theme::PRESETS {
        output.push_str(&format!("  {:<10} {}\n", preset.name, preset.description));
    }
    render::stdout(output).map_err(ThemeCommandError::Output)
}

fn preview(args: &[String]) -> Result<(), ThemeCommandError> {
    let name = args.first().ok_or(ThemeCommandError::MissingTheme)?;
    if args.len() > 1 {
        return Err(ThemeCommandError::TooManyArguments);
    }

    let rendered =
        theme::preview(name, io::stdout().is_terminal()).map_err(ThemeCommandError::Theme)?;
    render::stdout(format!("Theme: {name}\n{rendered}\n")).map_err(ThemeCommandError::Output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_and_unknown_subcommands() {
        assert_eq!(
            theme(&[]).unwrap_err().to_string(),
            "theme: expected 'list' or 'preview <name>'"
        );
        assert_eq!(
            theme(&["show".into()]).unwrap_err().to_string(),
            "theme: unknown action 'show' (expected 'list' or 'preview')"
        );
    }

    #[test]
    fn reports_missing_preview_name() {
        assert_eq!(
            theme(&["preview".into()]).unwrap_err().to_string(),
            "theme: missing theme name after 'preview'"
        );
    }
}
