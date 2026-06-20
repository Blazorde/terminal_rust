use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::errors::rm::RmError;
use crate::render;

#[derive(Default)]
struct RmOptions {
    empty_dir: bool,
    force: bool,
    interactive: bool,
    recursive: bool,
    verbose: bool,
}

pub fn rm(args: &[String]) -> Result<(), RmError> {
    let request = parse_args(args)?;
    remove_targets(&request.options, &request.targets, ask_confirmation)
}

struct RmRequest {
    options: RmOptions,
    targets: Vec<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<RmRequest, RmError> {
    let mut options = RmOptions::default();
    let mut targets = Vec::new();
    let mut parse_options = true;

    for arg in args {
        if parse_options && arg == "--" {
            parse_options = false;
            continue;
        }

        if parse_options && arg.starts_with("--") {
            parse_long_option(arg, &mut options)?;
            continue;
        }

        if parse_options && is_short_option(arg) {
            parse_short_options(arg, &mut options)?;
            continue;
        }

        let target = arg.trim();
        if !target.is_empty() {
            targets.push(PathBuf::from(target));
        }
    }

    if targets.is_empty() {
        return Err(RmError::MissingTarget);
    }

    Ok(RmRequest { options, targets })
}

fn parse_long_option(option: &str, options: &mut RmOptions) -> Result<(), RmError> {
    match option {
        "--dir" => options.empty_dir = true,
        "--force" => options.force = true,
        "--interactive" => options.interactive = true,
        "--recursive" => options.recursive = true,
        "--verbose" => options.verbose = true,
        _ => return Err(RmError::InvalidOption(option.to_string())),
    }

    Ok(())
}

fn parse_short_options(option: &str, options: &mut RmOptions) -> Result<(), RmError> {
    for flag in option[1..].chars() {
        match flag {
            'd' => options.empty_dir = true,
            'f' => options.force = true,
            'i' => options.interactive = true,
            'r' | 'R' => options.recursive = true,
            'v' => options.verbose = true,
            _ => return Err(RmError::InvalidOption(format!("-{flag}"))),
        }
    }

    Ok(())
}

fn is_short_option(arg: &str) -> bool {
    arg.len() > 1 && arg.starts_with('-') && arg != "-"
}

fn remove_targets(
    options: &RmOptions,
    targets: &[PathBuf],
    confirm: fn(&str) -> io::Result<bool>,
) -> Result<(), RmError> {
    for target in targets {
        remove_target(options, target, confirm)?;
    }

    Ok(())
}

fn remove_target(
    options: &RmOptions,
    path: &Path,
    confirm: fn(&str) -> io::Result<bool>,
) -> Result<(), RmError> {
    if !path.exists() {
        return if options.force {
            Ok(())
        } else {
            Err(RmError::NotFound(path.to_path_buf()))
        };
    }

    if should_confirm(options, path) && !confirm_removal(path, options, confirm)? {
        return Ok(());
    }

    if path.is_dir() {
        return remove_directory(options, path);
    }

    fs::remove_file(path).map_err(|source| RmError::DeleteFailed {
        path: path.to_path_buf(),
        source,
    })?;

    print_removed(options, path)?;
    Ok(())
}

fn remove_directory(options: &RmOptions, path: &Path) -> Result<(), RmError> {
    if options.recursive {
        fs::remove_dir_all(path).map_err(|source| RmError::DeleteFailed {
            path: path.to_path_buf(),
            source,
        })?;
    } else if options.empty_dir {
        fs::remove_dir(path).map_err(|source| RmError::DeleteFailed {
            path: path.to_path_buf(),
            source,
        })?;
    } else {
        return Err(RmError::IsDirectory(path.to_path_buf()));
    }

    print_removed(options, path)?;
    Ok(())
}

fn should_confirm(options: &RmOptions, path: &Path) -> bool {
    options.interactive || is_dangerous_removal(options, path)
}

fn is_dangerous_removal(options: &RmOptions, path: &Path) -> bool {
    options.recursive && path.is_dir()
}

fn confirm_removal(
    path: &Path,
    options: &RmOptions,
    confirm: fn(&str) -> io::Result<bool>,
) -> Result<bool, RmError> {
    let message = if is_dangerous_removal(options, path) {
        format!(
            "rm: recursively remove directory '{}'? [y/N] ",
            path.display()
        )
    } else {
        format!("rm: remove '{}'? [y/N] ", path.display())
    };

    confirm(&message).map_err(|source| RmError::DeleteFailed {
        path: path.to_path_buf(),
        source,
    })
}

fn ask_confirmation(message: &str) -> io::Result<bool> {
    render::stdout(message)?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;

    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}

fn print_removed(options: &RmOptions, path: &Path) -> Result<(), RmError> {
    if options.verbose {
        render::stdout_line(format_args!("removed '{}'", path.display())).map_err(|source| {
            RmError::DeleteFailed {
                path: path.to_path_buf(),
                source,
            }
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::path::temp_path;

    fn confirm_yes(_message: &str) -> io::Result<bool> {
        Ok(true)
    }

    fn confirm_no(_message: &str) -> io::Result<bool> {
        Ok(false)
    }

    #[test]
    fn rm_removes_file() {
        let path = temp_path("rm", "file");
        fs::write(&path, "hello").expect("test file should be written");

        rm(&[path.to_string_lossy().into_owned()]).expect("rm should remove file");

        assert!(!path.exists());
    }

    #[test]
    fn rm_requires_target() {
        let error = rm(&[]).expect_err("rm should require a target");

        assert!(matches!(error, RmError::MissingTarget));
    }

    #[test]
    fn rm_rejects_directory_without_option() {
        let path = temp_path("rm", "directory");
        fs::create_dir(&path).expect("test directory should be created");

        let error = rm(&[path.to_string_lossy().into_owned()])
            .expect_err("rm should reject directory without option");

        assert!(matches!(error, RmError::IsDirectory(directory) if directory == path));

        fs::remove_dir(path).expect("test directory should be removed");
    }

    #[test]
    fn rm_removes_empty_directory_with_dir_option() {
        let path = temp_path("rm", "empty-dir");
        fs::create_dir(&path).expect("test directory should be created");

        rm(&["-d".to_string(), path.to_string_lossy().into_owned()])
            .expect("rm -d should remove empty directory");

        assert!(!path.exists());
    }

    #[test]
    fn rm_recursive_asks_confirmation() {
        let path = temp_path("rm", "recursive");
        fs::create_dir(&path).expect("test directory should be created");
        fs::write(path.join("file.txt"), "hello").expect("test file should be written");

        let options = RmOptions {
            recursive: true,
            ..RmOptions::default()
        };

        remove_targets(&options, std::slice::from_ref(&path), confirm_yes)
            .expect("recursive rm should remove directory after confirmation");

        assert!(!path.exists());
    }

    #[test]
    fn rm_recursive_stops_when_confirmation_is_rejected() {
        let path = temp_path("rm", "recursive-no");
        fs::create_dir(&path).expect("test directory should be created");
        fs::write(path.join("file.txt"), "hello").expect("test file should be written");

        let options = RmOptions {
            recursive: true,
            ..RmOptions::default()
        };

        remove_targets(&options, std::slice::from_ref(&path), confirm_no)
            .expect("rejected confirmation should not be an error");

        assert!(path.exists());

        fs::remove_dir_all(path).expect("test directory should be removed");
    }
}
