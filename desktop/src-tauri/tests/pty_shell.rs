use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

fn wait_for_prompt_frame(
    receiver: &Receiver<Vec<u8>>,
    output: &mut Vec<u8>,
    prompt_count: usize,
    writer: &mut dyn Write,
    cursor_answered: &mut bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
            output.extend(chunk);
        }
        if cfg!(windows) && !*cursor_answered && output.windows(4).any(|bytes| bytes == b"\x1b[6n")
        {
            writer
                .write_all(b"\x1b[1;1R")
                .and_then(|()| writer.flush())
                .expect("headless PTY should answer the cursor position query");
            *cursor_answered = true;
        }
        if String::from_utf8_lossy(output).matches("]777;").count() >= prompt_count {
            return;
        }
    }
    panic!(
        "desktop shell did not render prompt frame {prompt_count}: {}",
        String::from_utf8_lossy(output)
    );
}

fn wait_for_output(
    receiver: &Receiver<Vec<u8>>,
    output: &mut Vec<u8>,
    needle: &str,
    writer: &mut dyn Write,
    cursor_answered: &mut bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
            output.extend(chunk);
        }
        if cfg!(windows) && !*cursor_answered && output.windows(4).any(|bytes| bytes == b"\x1b[6n")
        {
            writer
                .write_all(b"\x1b[1;1R")
                .and_then(|()| writer.flush())
                .expect("headless PTY should answer the cursor position query");
            *cursor_answered = true;
        }
        if String::from_utf8_lossy(output).contains(needle) {
            return;
        }
    }
    panic!(
        "desktop shell did not render {needle:?}: {}",
        String::from_utf8_lossy(output)
    );
}

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
    command.env("TERM", "xterm-256color");
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
        output.contains("]777;") && output.contains("\"left\"") && output.contains("\"right\""),
        "typed prompt metadata was missing from PTY output: {output}"
    );
    for kind in [
        "user", "host", "path", "status", "runtime", "duration", "time",
    ] {
        assert!(
            output.contains(&format!("\"{kind}\"")),
            "{kind} prompt segment was missing from PTY output: {output}"
        );
    }
}

#[test]
fn live_dashboard_exits_and_restores_the_desktop_pty() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 30,
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
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(length) => {
                    if sender.send(buffer[..length].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_terminal-rust-desktop"));
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");
    let mut child = pair
        .slave
        .spawn_command(command)
        .expect("desktop shell mode should start");
    drop(pair.slave);

    let mut output = Vec::new();
    let mut cursor_answered = false;
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        1,
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"dashboard\r")
        .and_then(|()| writer.flush())
        .expect("dashboard command should reach the PTY");
    wait_for_output(
        &receiver,
        &mut output,
        "Live refresh: 1s",
        writer.as_mut(),
        &mut cursor_answered,
    );

    writer
        .write_all(b"q")
        .and_then(|()| writer.flush())
        .expect("dashboard quit key should reach the PTY");
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        2,
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"pwd\r")
        .and_then(|()| writer.flush())
        .expect("shell should accept a command after the dashboard");
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        3,
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"exit\r")
        .and_then(|()| writer.flush())
        .expect("restored shell should exit");

    let deadline = Instant::now() + Duration::from_secs(10);
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("timed out shell should stop");
            panic!(
                "shell did not exit after dashboard cleanup: {}",
                String::from_utf8_lossy(&output)
            );
        }
        thread::sleep(Duration::from_millis(20));
    };

    assert!(status.success());
    let output = String::from_utf8_lossy(&output);
    assert!(output.contains("SYSTEM DASHBOARD"));
    assert!(
        output.contains("\u{1b}[?1049h"),
        "alternate screen was not entered"
    );
    assert!(
        output.contains("\u{1b}[?1049l"),
        "alternate screen was not restored"
    );
    assert!(output.matches("]777;").count() >= 3);
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

#[test]
#[ignore = "manual cross-platform performance probe"]
fn desktop_performance_probe_reports_three_trials() {
    for trial in 1..=3 {
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
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut buffer = [0; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(length) => {
                        if sender.send(buffer[..length].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let mut command = CommandBuilder::new(env!("CARGO_BIN_EXE_terminal-rust-desktop"));
        command.arg("--pty-shell");
        command.env("TERMINAL_RUST_GUI", "1");
        command.env("TERM", "xterm-256color");
        let started_at = Instant::now();
        let mut child = pair
            .slave
            .spawn_command(command)
            .expect("desktop shell mode should start");
        drop(pair.slave);

        let mut output = Vec::new();
        let mut cursor_answered = false;
        wait_for_prompt_frame(
            &receiver,
            &mut output,
            1,
            writer.as_mut(),
            &mut cursor_answered,
        );
        let startup = started_at.elapsed();

        let command_at = Instant::now();
        writer
            .write_all(b"pwd\r")
            .and_then(|()| writer.flush())
            .expect("performance command should reach the PTY");
        wait_for_prompt_frame(
            &receiver,
            &mut output,
            2,
            writer.as_mut(),
            &mut cursor_answered,
        );
        let round_trip = command_at.elapsed();

        writer
            .write_all(b"exit\r")
            .and_then(|()| writer.flush())
            .expect("performance session should exit");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child
                .try_wait()
                .expect("performance shell status should be readable")
                .is_some()
            {
                break;
            }
            if Instant::now() >= deadline {
                child
                    .kill()
                    .expect("timed out performance shell should stop");
                panic!("performance shell did not exit");
            }
            thread::sleep(Duration::from_millis(10));
        }

        println!(
            "trial={trial} prompt_startup_us={} command_round_trip_us={}",
            startup.as_micros(),
            round_trip.as_micros()
        );
    }
}
