//! Stand-ins for system programs: each writes its name and arguments, then exits
//! with a chosen status. The directory that holds them is the whole `PATH` of the
//! shell under test, so no real program can be started.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

use super::common::{Sandbox, finish};

pub struct Fakes {
    directory: PathBuf,
}

impl Fakes {
    pub fn new(sandbox: &Sandbox) -> Self {
        let directory = sandbox.home().join("fake programs");
        fs::create_dir_all(&directory).expect("the program directory should be created");
        Self { directory }
    }

    pub fn add(&self, name: &str, status: i32) {
        self.script(
            name,
            &format!("printf '%s\\n' \"{name} $*\"\nexit {status}\n"),
            &format!("echo {name} %*\r\nexit /b {status}\r\n"),
        );
    }

    /// Add a program from the body of its script, in `sh` and in batch.
    pub fn script(&self, name: &str, unix: &str, windows: &str) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = windows;
            let path = self.directory.join(name);
            fs::write(&path, format!("#!/bin/sh\n{unix}")).expect("the program should be written");
            fs::set_permissions(&path, fs::Permissions::from_mode(0o755))
                .expect("the program should be executable");
        }
        #[cfg(windows)]
        {
            let _ = unix;
            let path = self.directory.join(format!("{name}.cmd"));
            fs::write(path, format!("@echo off\r\n{windows}"))
                .expect("the program should be written");
        }
    }

    /// The shell, finding only the fake programs.
    pub fn shell(&self, sandbox: &Sandbox) -> Command {
        let mut command = sandbox.shell();
        command.env("PATH", &self.directory);
        command
    }

    /// Run `script` with `-c`.
    pub fn run(&self, sandbox: &Sandbox, script: &str) -> Output {
        let mut command = self.shell(sandbox);
        command.args(["-c", script]);
        finish(command, b"")
    }
}
