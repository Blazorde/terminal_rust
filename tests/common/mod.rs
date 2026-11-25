//! Helpers shared by the tests that start the shell binary.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const BINARY: &str = env!("CARGO_BIN_EXE_terminal_rust");

/// A shell that blocks fails its test instead of hanging the suite.
const TIME_LIMIT: Duration = Duration::from_secs(60);

/// A temporary directory with an empty home and a working directory, removed on drop.
pub struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    pub fn new(label: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("the system clock should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "terminal rust {label} {} {time} {}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let sandbox = Self { root };
        fs::create_dir_all(sandbox.home()).expect("the sandbox home should be created");
        fs::create_dir_all(sandbox.work()).expect("the working directory should be created");
        sandbox
    }

    pub fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    pub fn work(&self) -> PathBuf {
        self.root.join("work")
    }

    /// Write a file in the working directory.
    pub fn write(&self, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
        let path = self.work().join(name);
        fs::write(&path, content).expect("the fixture should be written");
        path
    }

    /// `program` started in the working directory, with no access to the real home.
    pub fn command(&self, program: impl AsRef<Path>) -> Command {
        let home = self.home();
        let mut command = Command::new(program.as_ref());
        command
            .current_dir(self.work())
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("APPDATA", home.join("appdata"))
            .env("LOCALAPPDATA", home.join("localappdata"))
            .env("USER", "terminal-rust-test")
            .env("USERNAME", "terminal-rust-test")
            .env_remove("TERMINAL_RUST_CONFIG")
            .env_remove("TERMINAL_RUST_FASTFETCH")
            .env_remove("TERMINAL_RUST_HISTORY_FILE")
            .env_remove("TERMINAL_RUST_NATIVE_UI");
        command
    }

    pub fn shell(&self) -> Command {
        self.command(BINARY)
    }

    /// Run the shell with `args` and an empty standard input.
    pub fn run(&self, args: &[&str]) -> Output {
        self.pipe(args, "")
    }

    /// Run the shell with `args`, `input` being its standard input.
    pub fn pipe(&self, args: &[&str], input: impl AsRef<[u8]>) -> Output {
        let mut command = self.shell();
        command.args(args);
        finish(command, input.as_ref())
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Run `command` to its end, feeding it `input`.
pub fn finish(mut command: Command, input: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the program should start");
    // A shell that rejects its arguments may leave before it reads anything.
    let _ = child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input);

    let deadline = Instant::now() + TIME_LIMIT;
    while child
        .try_wait()
        .expect("the status should be readable")
        .is_none()
    {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the program did not finish in time");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    child
        .wait_with_output()
        .expect("the output should be readable")
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n")
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).replace("\r\n", "\n")
}

/// Check the status and the whole standard output at once.
#[track_caller]
pub fn assert_run(output: &Output, status: i32, expected: &str) {
    assert_eq!(
        (output.status.code(), stdout(output).as_str()),
        (Some(status), expected),
        "stderr: {}",
        stderr(output)
    );
}
