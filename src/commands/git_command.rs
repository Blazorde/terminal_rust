use std::env;

use crate::git::{self, GitCommandError};
use crate::render;

pub fn git(args: &[String]) -> Result<(), GitCommandError> {
    let current_dir = env::current_dir().map_err(GitCommandError::CurrentDirectory)?;
    let output = git::execute_developer_command(&current_dir, args)?;

    if !output.stdout.is_empty() {
        render::stdout(&output.stdout).map_err(GitCommandError::Output)?;
    }
    if !output.stderr.is_empty() {
        render::stderr(&output.stderr).map_err(GitCommandError::Output)?;
    }

    if output.success {
        Ok(())
    } else {
        Err(GitCommandError::CommandFailed(output.status_code))
    }
}
