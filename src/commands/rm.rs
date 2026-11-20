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
    // Refuse before anything is removed, so one dangerous target cancels the command.
    for target in targets {
        ensure_not_protected(target)?;
    }

    let mut errors = Vec::new();
    for target in targets {
        if let Err(error) = remove_target(options, target, confirm) {
            errors.push(error);
        }
    }

    match errors.len() {
        0 => Ok(()),
        1 => Err(errors.remove(0)),
        count => {
            for error in &errors {
                let _ = render::stderr_line(error);
            }
            Err(RmError::Several(count))
        }
    }
}

/// Refuse `.`, `..` and the root directory, as GNU `rm` does.
fn ensure_not_protected(path: &Path) -> Result<(), RmError> {
    let text = path.to_string_lossy();
    let trimmed = text.trim_end_matches(['/', '\\']);
    let last = trimmed.rsplit(['/', '\\']).next().unwrap_or("");
    let reason = if last == "." || last == ".." {
        Some("'.' and '..' are not removable")
    } else if (trimmed.is_empty() && !text.is_empty())
        || fs::canonicalize(path).is_ok_and(|resolved| resolved.parent().is_none())
    {
        Some("it is the root directory")
    } else {
        None
    };

    match reason {
        Some(reason) => Err(RmError::Refused {
            path: path.to_path_buf(),
            reason,
        }),
        None => Ok(()),
    }
}

/// Look at the path itself: a symlink is a file, even when it points to a
/// directory or to nothing.
fn exists_without_following(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

fn remove_target(
    options: &RmOptions,
    path: &Path,
    confirm: fn(&str) -> io::Result<bool>,
) -> Result<(), RmError> {
    if !exists_without_following(path) {
        return if options.force {
            Ok(())
        } else {
            Err(RmError::NotFound(path.to_path_buf()))
        };
    }

    if should_confirm(options, path) && !confirm_removal(path, options, confirm)? {
        return Ok(());
    }

    if is_real_directory(path) {
        return remove_directory(options, path);
    }

    remove_file_or_link(path).map_err(|source| RmError::DeleteFailed {
        path: path.to_path_buf(),
        source,
    })?;

    print_removed(options, path)?;
    Ok(())
}

fn remove_file_or_link(path: &Path) -> io::Result<()> {
    let removed = fs::remove_file(path);
    // Windows removes a directory symlink with remove_dir.
    if removed.is_err() && fs::metadata(path).is_ok_and(|metadata| metadata.is_dir()) {
        return fs::remove_dir(path);
    }
    removed
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
    options.interactive || (!options.force && is_dangerous_removal(options, path))
}

fn is_dangerous_removal(options: &RmOptions, path: &Path) -> bool {
    options.recursive && is_real_directory(path)
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
    // The question is captured like any output; show it before waiting.
    render::flush_captured();

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

    fn confirm_unexpected(_message: &str) -> io::Result<bool> {
        panic!("forced removal must not request confirmation")
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

    #[test]
    fn rm_recursive_force_skips_confirmation() {
        let path = temp_path("rm", "recursive-force");
        fs::create_dir(&path).expect("test directory should be created");
        fs::write(path.join("file.txt"), "hello").expect("test file should be written");

        let options = RmOptions {
            force: true,
            recursive: true,
            ..RmOptions::default()
        };

        remove_targets(&options, std::slice::from_ref(&path), confirm_unexpected)
            .expect("rm -rf should remove the directory without prompting");

        assert!(!path.exists());
    }

    #[test]
    fn rm_interactive_still_confirms_even_with_force() {
        let path = temp_path("rm", "interactive-force");
        fs::write(&path, "hello").expect("test file should be written");

        let options = RmOptions {
            force: true,
            interactive: true,
            ..RmOptions::default()
        };

        remove_targets(&options, std::slice::from_ref(&path), confirm_no)
            .expect("declined interactive removal should not fail");

        assert!(path.exists());
        fs::remove_file(path).expect("test file should be removed");
    }

    #[test]
    fn dot_dot_dot_and_the_root_are_protected() {
        for target in [
            ".", "..", "./", "../", "a/..", "a/.", "a/../", "/", "//", "/.",
        ] {
            let result = ensure_not_protected(Path::new(target));

            assert!(
                matches!(result, Err(RmError::Refused { .. })),
                "{target:?} should be refused"
            );
        }
    }

    #[test]
    fn ordinary_names_are_not_protected() {
        for target in [
            "a",
            ".hidden",
            "..data",
            "a..b",
            "...",
            "dir/file.txt",
            "dir/.git",
        ] {
            assert!(
                ensure_not_protected(Path::new(target)).is_ok(),
                "{target:?} should be allowed"
            );
        }
    }

    #[test]
    fn a_protected_target_cancels_the_whole_command() {
        let file = temp_path("rm", "guard-first");
        fs::write(&file, "keep me").expect("test file should be written");
        // Recursive without force asks before removing a directory, and the
        // answer is no, so this test cannot delete anything even if the guard broke.
        let options = RmOptions {
            recursive: true,
            ..RmOptions::default()
        };

        let result = remove_targets(&options, &[file.clone(), PathBuf::from(".")], confirm_no);

        assert!(matches!(result, Err(RmError::Refused { .. })));
        assert!(
            file.exists(),
            "nothing should be removed before the refusal"
        );
        fs::remove_file(file).expect("test file should be removed");
    }

    #[test]
    fn rm_continues_with_the_other_targets_after_a_failure() {
        let first = temp_path("rm", "continue-first");
        let last = temp_path("rm", "continue-last");
        let missing = temp_path("rm", "continue-missing");
        fs::write(&first, "a").expect("test file should be written");
        fs::write(&last, "c").expect("test file should be written");

        let result = rm(&[
            first.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
            last.to_string_lossy().into_owned(),
        ]);

        assert!(matches!(result, Err(RmError::NotFound(path)) if path == missing));
        assert!(!first.exists() && !last.exists());
    }

    #[test]
    fn rm_force_ignores_missing_targets_and_still_removes_the_rest() {
        let present = temp_path("rm", "force-present");
        let missing = temp_path("rm", "force-missing");
        fs::write(&present, "a").expect("test file should be written");

        rm(&[
            "-f".to_string(),
            missing.to_string_lossy().into_owned(),
            present.to_string_lossy().into_owned(),
        ])
        .expect("rm -f should ignore the missing target");

        assert!(!present.exists());
    }

    #[test]
    fn several_failures_are_all_printed_and_counted() {
        let first = temp_path("rm", "several-first");
        let second = temp_path("rm", "several-second");

        let result = rm(&[
            first.to_string_lossy().into_owned(),
            second.to_string_lossy().into_owned(),
        ]);

        assert!(matches!(result, Err(RmError::Several(2))));
    }

    #[cfg(unix)]
    #[test]
    fn a_broken_symlink_can_be_removed() {
        let link = temp_path("rm", "broken-link");
        std::os::unix::fs::symlink(temp_path("rm", "nowhere"), &link)
            .expect("symlink should be created");

        rm(&[link.to_string_lossy().into_owned()]).expect("a broken link should be removable");

        assert!(fs::symlink_metadata(&link).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_symlink_to_a_directory_leaves_the_directory_alone() {
        let directory = temp_path("rm", "link-target");
        let link = temp_path("rm", "link-to-directory");
        fs::create_dir(&directory).expect("test directory should be created");
        fs::write(directory.join("keep.txt"), "keep").expect("test file should be written");
        std::os::unix::fs::symlink(&directory, &link).expect("symlink should be created");
        let options = RmOptions {
            force: true,
            recursive: true,
            ..RmOptions::default()
        };

        remove_targets(&options, std::slice::from_ref(&link), confirm_unexpected)
            .expect("the link should be removed without confirmation");

        assert!(fs::symlink_metadata(&link).is_err());
        assert!(directory.join("keep.txt").exists());
        fs::remove_dir_all(directory).expect("test directory should be removed");
    }
}
