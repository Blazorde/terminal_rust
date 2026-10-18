use std::env;

use crate::git::{self, GitCommandError};
pub fn git(args: &[String]) -> Result<(), GitCommandError> {
    let current_dir = env::current_dir().map_err(GitCommandError::CurrentDirectory)?;
    git::run_native_command(&current_dir, args).map(|_| ())
}
