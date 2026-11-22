use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// An empty home for the shell under test, so it never reads the developer's own
/// configuration and the results do not depend on whose machine they run on.
fn isolated_home() -> std::path::PathBuf {
    let home = std::env::temp_dir().join("terminal-rust-tests-isolated-home");
    fs::create_dir_all(&home).expect("the isolated home should be created");
    home
}

fn isolate(command: &mut CommandBuilder) {
    let home = isolated_home();
    command.env("HOME", &home);
    command.env("USERPROFILE", &home);
    command.env("XDG_CONFIG_HOME", home.join("config"));
    command.env("XDG_DATA_HOME", home.join("data"));
    command.env("APPDATA", home.join("appdata"));
    command.env("LOCALAPPDATA", home.join("localappdata"));
    command.env_remove("TERMINAL_RUST_CONFIG");
}

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

/// Wait until the prompt that follows `marker` has been drawn.
///
/// Every keystroke redraws the prompt, so counting prompt frames cannot tell
/// that a command has finished. The line editor also discards input typed
/// before it starts reading again, hence the short settling delay.
fn wait_for_prompt_after(
    receiver: &Receiver<Vec<u8>>,
    output: &mut Vec<u8>,
    marker: &str,
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
        let text = String::from_utf8_lossy(output);
        if text
            .rfind(marker)
            .is_some_and(|index| text[index + marker.len()..].contains("]777;"))
        {
            thread::sleep(Duration::from_millis(150));
            return;
        }
    }
    panic!(
        "desktop shell did not draw a prompt after {marker:?}: {}",
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

fn wait_for_output_occurrences(
    receiver: &Receiver<Vec<u8>>,
    output: &mut Vec<u8>,
    needle: &str,
    occurrences: usize,
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
        if String::from_utf8_lossy(output).matches(needle).count() >= occurrences {
            return;
        }
    }
    panic!(
        "desktop shell did not render {needle:?} {occurrences} times: {:?}",
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
    isolate(&mut command);
    command.arg("--pty-shell");
    command.cwd(env!("CARGO_MANIFEST_DIR"));
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");
    command.env("USER", "terminal-rust-test");
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
        .write_all(b"help\r")
        .and_then(|()| writer.flush())
        .expect("help command should reach the PTY");
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        2,
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"echo multiline `\r")
        .and_then(|()| writer.flush())
        .expect("multiline command should reach the PTY");
    // Type the next line only once the continuation prompt is shown: the line
    // editor discards input typed ahead while it re-arms the terminal.
    wait_for_output(
        &receiver,
        &mut output,
        ">>",
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"line\r")
        .and_then(|()| writer.flush())
        .expect("continuation line should reach the PTY");
    wait_for_output(
        &receiver,
        &mut output,
        "multiline line",
        writer.as_mut(),
        &mut cursor_answered,
    );
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
        .expect("exit command should reach the PTY");

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
    while let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
        output.extend(chunk);
    }
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
    assert!(
        ["[36m", "[35m", "[1;36m", "[1;35m"]
            .iter()
            .any(|code| output.contains(&format!("\u{1b}{code}"))),
        "colored prompt styling was missing from PTY output: {output}"
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
fn interactive_pty_keeps_history_directory_changes_and_long_line_edits() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow the Unix epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "terminal-rust-editing-{}-{nonce}",
        std::process::id()
    ));
    let target = root.join("target");
    fs::create_dir_all(&target).expect("PTY fixture directory should be created");

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
    isolate(&mut command);
    command.arg("--pty-shell");
    command.cwd(&root);
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
        .write_all(b"echo history-check\r")
        .and_then(|()| writer.flush())
        .expect("first history command should reach the PTY");
    wait_for_output_occurrences(
        &receiver,
        &mut output,
        "history-check",
        2,
        writer.as_mut(),
        &mut cursor_answered,
    );
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        2,
        writer.as_mut(),
        &mut cursor_answered,
    );
    let first_history_count = String::from_utf8_lossy(&output)
        .matches("history-check")
        .count();

    writer
        .write_all(b"\x1b[A\r")
        .and_then(|()| writer.flush())
        .expect("history navigation should reach the PTY");
    wait_for_output_occurrences(
        &receiver,
        &mut output,
        "history-check",
        first_history_count + 2,
        writer.as_mut(),
        &mut cursor_answered,
    );

    wait_for_prompt_frame(
        &receiver,
        &mut output,
        3,
        writer.as_mut(),
        &mut cursor_answered,
    );

    let cd_command = format!("cd \"{}\"\r", target.display());
    writer
        .write_all(cd_command.as_bytes())
        .and_then(|()| writer.flush())
        .expect("cd command should reach the PTY");
    wait_for_prompt_frame(
        &receiver,
        &mut output,
        4,
        writer.as_mut(),
        &mut cursor_answered,
    );
    writer
        .write_all(b"pwd\r")
        .and_then(|()| writer.flush())
        .expect("pwd after cd should reach the PTY");
    wait_for_output_occurrences(
        &receiver,
        &mut output,
        &target.display().to_string(),
        1,
        writer.as_mut(),
        &mut cursor_answered,
    );

    wait_for_prompt_frame(
        &receiver,
        &mut output,
        5,
        writer.as_mut(),
        &mut cursor_answered,
    );

    let body = "a".repeat(110);
    let expected = format!("{}Xa", "a".repeat(109));
    let mut long_command = format!("echo {body}").into_bytes();
    long_command.extend_from_slice(b"\x1b[DX\r");
    writer
        .write_all(&long_command)
        .and_then(|()| writer.flush())
        .expect("long edited command should reach the PTY");
    wait_for_output_occurrences(
        &receiver,
        &mut output,
        &expected,
        1,
        writer.as_mut(),
        &mut cursor_answered,
    );

    // On Unix the command output starts a line, whereas the line being edited
    // is prefixed by "echo ", so the text alone is not proof that it has run.
    let output_marker = if cfg!(windows) {
        expected.clone()
    } else {
        format!("\n{expected}")
    };
    wait_for_prompt_after(
        &receiver,
        &mut output,
        &output_marker,
        writer.as_mut(),
        &mut cursor_answered,
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        if Instant::now() >= deadline {
            child.kill().expect("timed out shell should stop");
            panic!(
                "interactive PTY did not exit: {}",
                String::from_utf8_lossy(&output)
            );
        }
        if Instant::now() >= next_exit {
            // Input typed while the line editor re-arms the terminal is
            // discarded, so type `exit` again until the shell has quit. The
            // shell may quit between the status check and the write.
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    };

    drop(writer);
    drop(pair.master);
    assert!(
        status.success(),
        "interactive PTY shell should exit cleanly"
    );
    fs::remove_dir_all(root).expect("PTY fixture directory should be removed");
}

// rustyline keeps lines typed ahead only on Unix (its `buffer-redux` feature), so
// this guards a Unix behavior: pasted lines must all run, in order.
#[cfg(unix)]
#[test]
fn lines_pasted_in_one_write_all_run_in_order() {
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
    isolate(&mut command);
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
        .write_all(b"echo x1\recho x2\recho x3\r")
        .and_then(|()| writer.flush())
        .expect("pasted lines should reach the PTY");

    // Every keystroke redraws the prompt, so a slow machine needs time to get
    // through the whole paste. A command's output starts a line; its typed echo
    // does not.
    let results = ["\r\nx1\r\n", "\r\nx2\r\n", "\r\nx3\r\n"];
    let paste_deadline = Instant::now() + Duration::from_secs(30);
    let text = loop {
        if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
            output.extend(chunk);
        }
        let text = String::from_utf8_lossy(&output).into_owned();
        if results.iter().all(|result| text.contains(result)) {
            break text;
        }
        assert!(
            Instant::now() < paste_deadline,
            "pasted commands did not all run: {text}"
        );
    };
    let positions: Vec<usize> = results
        .iter()
        .map(|result| text.find(result).unwrap_or(usize::MAX))
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "pasted commands should run in order: {text}"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        assert!(Instant::now() < deadline, "shell did not exit: {text}");
        if Instant::now() >= next_exit {
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "shell should exit cleanly");
}

// `!!`, `!$` and `!prefix` stand for earlier lines of the prompt: the expanded
// line runs and is recorded, and a missing event runs nothing.
#[cfg(unix)]
#[test]
fn history_references_expand_at_the_prompt() {
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
    isolate(&mut command);
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

    // A command's output starts a line; its typed echo does not.
    let mut type_line = |line: &str, needle: &str, occurrences: usize, output: &mut Vec<u8>| {
        writer
            .write_all(format!("{line}\r").as_bytes())
            .and_then(|()| writer.flush())
            .expect("a line should reach the PTY");
        wait_for_output_occurrences(
            &receiver,
            output,
            needle,
            occurrences,
            writer.as_mut(),
            &mut cursor_answered,
        );
    };
    type_line("echo first-line", "\r\nfirst-line\r\n", 1, &mut output);
    type_line("!!", "\r\nfirst-line\r\n", 2, &mut output);
    type_line("echo a b zed", "\r\na b zed\r\n", 1, &mut output);
    type_line("echo !$", "\r\nzed\r\n", 1, &mut output);
    type_line("!nothing", "!nothing: event not found", 1, &mut output);
    type_line("echo after-error", "\r\nafter-error\r\n", 1, &mut output);
    type_line("history", "    4  echo after-error\r\n", 1, &mut output);

    let text = String::from_utf8_lossy(&output).into_owned();
    // The expanded line is what is recorded; a repeat and a missing event are not.
    for recorded in [
        "    1  echo first-line",
        "    2  echo a b zed",
        "    3  echo zed",
    ] {
        assert!(text.contains(recorded), "{recorded:?} in {text}");
    }
    assert!(!text.contains("    5  echo after-error"), "{text}");
    assert!(
        !text.contains("\r\nnothing"),
        "a missing event must run nothing: {text}"
    );

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        assert!(Instant::now() < deadline, "shell did not exit: {text}");
        if Instant::now() >= next_exit {
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "shell should exit cleanly");
}

// While a program runs the terminal is in cooked mode and turns Enter into a
// line feed, so input typed ahead reaches the line editor as Ctrl+J. It must
// still accept the line: a binding that inserted a newline instead left the
// command unexecuted.
#[cfg(unix)]
#[test]
fn a_command_typed_while_a_program_runs_is_executed_afterwards() {
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
    isolate(&mut command);
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
        .write_all(b"sleep 1\r")
        .and_then(|()| writer.flush())
        .expect("the program command should reach the PTY");
    // Type the next command while `sleep` is still running.
    thread::sleep(Duration::from_millis(400));
    writer
        .write_all(b"echo typed-ahead\r")
        .and_then(|()| writer.flush())
        .expect("the typed-ahead command should reach the PTY");

    // The command's output starts a line, unlike its typed echo.
    let result = "\r\ntyped-ahead\r\n";
    let deadline = Instant::now() + Duration::from_secs(30);
    let text = loop {
        if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
            output.extend(chunk);
        }
        let text = String::from_utf8_lossy(&output).into_owned();
        if text.contains(result) {
            break text;
        }
        assert!(
            Instant::now() < deadline,
            "the command typed while a program ran was not executed: {text}"
        );
    };

    let exit_deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        assert!(Instant::now() < exit_deadline, "shell did not exit: {text}");
        if Instant::now() >= next_exit {
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "shell should exit cleanly");
}

// A configured process command used to run with its output captured and no
// standard input, so an interactive program saw an empty input. In a terminal it
// must read what the user types.
#[cfg(unix)]
#[test]
fn a_process_command_reads_what_the_user_types() {
    let directory = std::env::temp_dir().join(format!(
        "terminal-rust-process-stdin-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos()
    ));
    fs::create_dir(&directory).expect("fixture directory should be created");
    let config = directory.join("config.toml");
    fs::write(
        &config,
        "version = 1\n[commands.ask]\nkind = \"process\"\ntarget = \"sh\"\nargs = [\"-c\", \"read answer; echo got:$answer\"]\nappend_args = false\n",
    )
    .expect("configuration should be written");

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
    isolate(&mut command);
    command.arg("--pty-shell");
    command.env("TERMINAL_RUST_GUI", "1");
    command.env("TERM", "xterm-256color");
    command.env("TERMINAL_RUST_CONFIG", &config);
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
        .write_all(b"ask\r")
        .and_then(|()| writer.flush())
        .expect("the command should reach the PTY");
    // Give the program time to start and wait for its input.
    thread::sleep(Duration::from_millis(500));
    writer
        .write_all(b"hello\r")
        .and_then(|()| writer.flush())
        .expect("the answer should reach the PTY");

    let result = "\r\ngot:hello\r\n";
    let deadline = Instant::now() + Duration::from_secs(30);
    let text = loop {
        if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
            output.extend(chunk);
        }
        let text = String::from_utf8_lossy(&output).into_owned();
        if text.contains(result) {
            break text;
        }
        assert!(
            Instant::now() < deadline,
            "the program did not receive what was typed: {text}"
        );
    };

    let exit_deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("shell status should be readable") {
            break status;
        }
        assert!(Instant::now() < exit_deadline, "shell did not exit: {text}");
        if Instant::now() >= next_exit {
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert!(status.success(), "shell should exit cleanly");
    fs::remove_dir_all(directory).expect("fixture directory should be removed");
}

/// `Ctrl+Z` stops the program that runs and not the shell; `jobs`, `bg` and `fg` then
/// continue it, and `exit` warns once about a job that is still stopped (ADR-0024).
#[cfg(unix)]
#[test]
fn ctrl_z_stops_the_program_and_bg_and_fg_continue_it() {
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
    isolate(&mut command);
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

    // What was written since the last step, so that each step waits for its own answer.
    let mut step = |input: &[u8], expected: &str| {
        let from = output.len();
        writer
            .write_all(input)
            .and_then(|()| writer.flush())
            .expect("the input should reach the PTY");
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Ok(chunk) = receiver.recv_timeout(Duration::from_millis(20)) {
                output.extend(chunk);
            }
            let text = String::from_utf8_lossy(&output[from..]).into_owned();
            if text.contains(expected) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "expected {expected:?} after {:?}, got: {text:?}",
                String::from_utf8_lossy(input)
            );
        }
    };

    step(b"sleep 60\r", "sleep 60");
    // The program has the terminal: `Ctrl+Z` stops it and the shell says so.
    thread::sleep(Duration::from_millis(500));
    step(b"\x1a", "[1]+  Stopped  sleep 60");
    step(b"jobs\r", "[1]+  Stopped  sleep 60\r\n");
    step(b"bg\r", "[1]+  sleep 60 &");
    step(b"jobs\r", "[1]+  Running  sleep 60");
    // `fg` gives the terminal back to the program, so `Ctrl+C` ends it and not the shell.
    step(b"fg\r", "sleep 60\r\n");
    thread::sleep(Duration::from_millis(300));
    step(b"\x03", "fg: exited with status 130");
    step(b"echo after-$?\r", "after-130");
    // A job that is stopped again blocks the first `exit`.
    step(b"sleep 60\r", "sleep 60");
    thread::sleep(Duration::from_millis(500));
    step(b"\x1a", "Stopped  sleep 60");
    step(b"trap 'echo trap-ran' EXIT\r", "trap 'echo trap-ran' EXIT");
    step(b"exit\r", "There are stopped jobs.");

    let exit_deadline = Instant::now() + Duration::from_secs(10);
    let mut next_exit = Instant::now();
    loop {
        if child
            .try_wait()
            .expect("shell status should be readable")
            .is_some()
        {
            break;
        }
        assert!(
            Instant::now() < exit_deadline,
            "the second exit did not leave"
        );
        if Instant::now() >= next_exit {
            let _ = writer.write_all(b"exit\r").and_then(|()| writer.flush());
            next_exit = Instant::now() + Duration::from_millis(500);
        }
        thread::sleep(Duration::from_millis(20));
    }

    // The `EXIT` trap ran as the shell ended, at the prompt of the editor too.
    let drain_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < drain_deadline {
        match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(chunk) => output.extend(chunk),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
    let text = String::from_utf8_lossy(&output).into_owned();
    assert!(
        text.contains("\r\ntrap-ran\r\n"),
        "the exit trap did not run: {text:?}"
    );
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
    isolate(&mut command);
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
        let home = isolated_home();
        Command::new(env!("CARGO_BIN_EXE_terminal-rust-desktop"))
            .env("HOME", &home)
            .env("USERPROFILE", &home)
            .env("XDG_CONFIG_HOME", home.join("config"))
            .env("XDG_DATA_HOME", home.join("data"))
            .env("APPDATA", home.join("appdata"))
            .env("LOCALAPPDATA", home.join("localappdata"))
            .env_remove("TERMINAL_RUST_CONFIG")
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
        isolate(&mut command);
        isolate(&mut command);
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
