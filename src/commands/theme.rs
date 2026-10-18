use std::error::Error;
use std::fmt;
use std::io::{self, IsTerminal};

use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};

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

    let is_terminal = io::stdout().is_terminal();
    let rendered = theme::preview(name, is_terminal).map_err(ThemeCommandError::Theme)?;
    if is_terminal {
        execute!(io::stdout(), MoveTo(0, 0), Clear(ClearType::All))
            .map_err(ThemeCommandError::Output)?;
    }
    render::stdout(format!(
        "Theme: {name}\n\nExample session (simulated; commands are not run):\n{rendered}"
    ))
    .map_err(ThemeCommandError::Output)
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

    #[test]
    fn preview_keeps_a_full_demo_when_output_is_not_a_terminal() {
        let rendered = theme::preview("minimal", false).expect("minimal preview should render");

        assert!(rendered.contains("ls"));
        assert!(rendered.contains("git status --short"));
        assert!(rendered.contains("cargo test --lib"));
        assert!(rendered.contains("cat: missing.txt"));
        assert!(!rendered.contains("\u{1b}["));
    }
}
