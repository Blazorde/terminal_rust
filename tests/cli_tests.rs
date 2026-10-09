use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "terminal rust {label} {} {unique}",
            std::process::id()
        ));

        fs::create_dir(&path).expect("the temporary test directory should be created");

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run_cli(current_dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_terminal_rust"))
        .args(args)
        .current_dir(current_dir)
        .output()
        .expect("the terminal_rust binary should start")
}

fn run_interactive(current_dir: &Path, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_terminal_rust"))
        .current_dir(current_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the terminal_rust binary should start");

    child
        .stdin
        .as_mut()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("interactive input should be written");

    child
        .wait_with_output()
        .expect("the terminal_rust binary should finish")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be valid UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr should be valid UTF-8")
}

#[test]
fn pwd_prints_the_process_working_directory() {
    let directory = TestDirectory::new("pwd");
    let output = run_cli(directory.path(), &["pwd"]);

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());

    let reported_path = PathBuf::from(stdout(&output).trim());
    assert_eq!(
        fs::canonicalize(reported_path).expect("the reported directory should exist"),
        fs::canonicalize(directory.path()).expect("the test directory should exist")
    );
}

#[test]
fn ls_lists_files_and_directories_from_the_working_directory() {
    let directory = TestDirectory::new("ls");
    fs::write(directory.path().join("alpha.txt"), "alpha")
        .expect("the fixture file should be created");
    fs::create_dir(directory.path().join("nested directory"))
        .expect("the fixture directory should be created");

    let output = run_cli(directory.path(), &["ls"]);
    let lines: Vec<String> = stdout(&output).lines().map(ToOwned::to_owned).collect();

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert!(output.stderr.is_empty());
    assert!(lines.contains(&"alpha.txt".to_string()));
    assert!(lines.contains(&"nested directory".to_string()));
}

#[test]
fn unknown_command_returns_a_failure_and_writes_to_stderr() {
    let directory = TestDirectory::new("unknown command");
    let output = run_cli(directory.path(), &["does-not-exist"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(stderr(&output).trim(), "unknown command: does-not-exist");
}

#[test]
fn missing_required_argument_returns_a_failure_and_writes_to_stderr() {
    let directory = TestDirectory::new("missing argument");
    let output = run_cli(directory.path(), &["cat"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(stderr(&output).trim(), "cat: missing file name");
}

#[test]
fn interactive_prompt_stays_simple_outside_git() {
    let directory = TestDirectory::new("prompt outside git");
    let output = run_interactive(directory.path(), "exit\n");
    let rendered = stdout(&output);
    let rendered_path = rendered
        .strip_suffix("> ")
        .expect("the simple prompt should end with its symbol");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        fs::canonicalize(rendered_path).expect("the rendered directory should exist"),
        fs::canonicalize(directory.path()).expect("the fixture directory should exist")
    );
}

#[test]
fn interactive_prompt_shows_a_dirty_git_branch() {
    let directory = TestDirectory::new("git prompt");
    let git_available = Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());

    if !git_available {
        return;
    }

    let initialized = Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(directory.path())
        .status()
        .expect("git should start");
    assert!(initialized.success());

    let named_branch = Command::new("git")
        .args(["symbolic-ref", "HEAD", "refs/heads/main"])
        .current_dir(directory.path())
        .status()
        .expect("git should name the initial branch");
    assert!(named_branch.success());

    fs::write(directory.path().join("dirty file.txt"), "dirty")
        .expect("the dirty fixture should be created");

    let output = run_interactive(directory.path(), "exit\n");
    let rendered = stdout(&output);
    let rendered_path = rendered
        .strip_suffix("  main* > ")
        .expect("the Git prompt should contain its branch and dirty marker");

    assert!(output.status.success(), "stderr: {}", stderr(&output));
    assert_eq!(
        fs::canonicalize(rendered_path).expect("the rendered directory should exist"),
        fs::canonicalize(directory.path()).expect("the fixture directory should exist")
    );
}
