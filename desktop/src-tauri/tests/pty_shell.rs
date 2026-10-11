use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[test]
fn desktop_binary_runs_the_real_shell_through_a_native_pty() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 100,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("native PTY should open");
    let mut reader = pair
        .master
        .try_clone_reader()
        .expect("PTY output should be readable");
    let mut writer = pair
        .master
        .take_writer()
        .expect("PTY input should be writable");

    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_terminal-rust-desktop"));
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    let mut child = pair
        .slave
        .spawn_command(command)
        .expect("desktop shell mode should start");
    drop(pair.slave);

    // Let rustyline switch the PTY into raw mode before reproducing xterm's
    // keyboard stream. Sending bytes during process bootstrap is racy on
    // Windows ConPTY and does not model an interactive user.
    std::thread::sleep(Duration::from_millis(300));
    // On Windows, rustyline asks an ANSI terminal for its cursor position
    // during startup. xterm answers this query; the headless test PTY must do
    // the same before sending key presses. Unix obtains the position from its
    // terminal interface and must not receive this synthetic response.
    let input: &[u8] = if cfg!(windows) {
        b"\x1b[1;1Rhelp\rexit\r"
    } else {
        b"help\rexit\r"
    };
    writer
        .write_all(input)
        .and_then(|()| writer.flush())
        .expect("commands should reach the PTY");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        if Instant::now() >= deadline {
            timed_out = true;
            child.kill().expect("timed out shell should stop");
            break child
                .wait()
                .expect("stopped shell status should be readable");
        }
        std::thread::sleep(Duration::from_millis(20));
    };

    drop(writer);
    drop(pair.master);
    let mut output = Vec::new();
    reader
        .read_to_end(&mut output)
        .expect("PTY output should remain readable until exit");
    let output = String::from_utf8_lossy(&output);

    assert!(
        !timed_out,
        "desktop shell did not exit after receiving the exit command: {output}"
    );
    assert!(
        status.success(),
        "desktop shell exited unsuccessfully: {output}"
    );
    assert!(
        output.contains("Available commands:"),
        "real shell help was missing from PTY output: {output}"
    );
    assert!(
        output.contains("\u{1b}]777;")
            && output.contains("\"left\"")
            && output.contains("\"right\""),
        "typed prompt metadata was missing from PTY output: {output}"
    );
    for kind in [
        "user", "host", "path", "status", "runtime", "duration", "time",
    ] {
        assert!(
            output.contains(&format!("\"kind\":\"{kind}\"")),
            "{kind} prompt segment was missing from PTY output: {output}"
        );
    }
}

#[test]
fn concurrent_desktop_sessions_keep_their_working_directories_isolated() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow the Unix epoch")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("terminal-rust-tabs-{}-{nonce}", std::process::id()));
    let first_dir = root.join("first session");
    let second_dir = root.join("second session");
    fs::create_dir_all(&first_dir).expect("first session directory should be created");
    fs::create_dir_all(&second_dir).expect("second session directory should be created");

    let spawn = |directory: &std::path::Path| {
        Command::new(env!("CARGO_BIN_EXE_terminal-rust-desktop"))
            .arg("--pty-shell")
            .current_dir(directory)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("desktop session should start")
    };
    let mut first = spawn(&first_dir);
    let mut second = spawn(&second_dir);
    first
        .stdin
        .as_mut()
        .expect("first session stdin should be open")
        .write_all(b"pwd\nexit\n")
        .expect("first session should receive commands");
    second
        .stdin
        .as_mut()
        .expect("second session stdin should be open")
        .write_all(b"pwd\nexit\n")
        .expect("second session should receive commands");

    let first_output = first.wait_with_output().expect("first session should exit");
    let second_output = second
        .wait_with_output()
        .expect("second session should exit");
    let first_stdout =
        String::from_utf8(first_output.stdout).expect("first output should be UTF-8");
    let second_stdout =
        String::from_utf8(second_output.stdout).expect("second output should be UTF-8");

    assert!(first_output.status.success());
    assert!(second_output.status.success());
    assert!(first_stdout.contains(&first_dir.display().to_string()));
    assert!(!first_stdout.contains(&second_dir.display().to_string()));
    assert!(second_stdout.contains(&second_dir.display().to_string()));
    assert!(!second_stdout.contains(&first_dir.display().to_string()));

    fs::remove_dir_all(root).expect("session fixtures should be removed");
}
