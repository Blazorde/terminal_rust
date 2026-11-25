//! How the binary is started: `-c`, script files, standard input and one command.

mod common;

use common::{Sandbox, assert_run, stderr, stdout};

#[test]
fn a_command_string_goes_on_after_a_failure_and_ends_with_the_last_status() {
    let sandbox = Sandbox::new("launch string");

    assert_run(
        &sandbox.run(&["-c", "false\necho after $?"]),
        0,
        "after 1\n",
    );
    assert_run(&sandbox.run(&["-c", "echo before\nfalse"]), 1, "before\n");
    assert_run(&sandbox.run(&["-c", ""]), 0, "");

    let unknown = sandbox.run(&["-c", "terminal-rust-no-such-program"]);
    assert_run(&unknown, 127, "");
    assert_eq!(
        stderr(&unknown),
        "unknown command: terminal-rust-no-such-program\n"
    );
}

#[test]
fn a_command_string_reads_its_name_and_arguments() {
    let sandbox = Sandbox::new("launch arguments");
    let script = "echo $0 $# $1\nfor word in \"$@\"; do echo \"<$word>\"; done\nshift\necho $1";

    assert_run(
        &sandbox.run(&["-c", script, "me", "a b", "c"]),
        0,
        "me 2 a b\n<a b>\n<c>\nc\n",
    );
    assert_run(&sandbox.run(&["-c", "echo $0 $#"]), 0, "terminal_rust 0\n");
    assert_run(
        &sandbox.run(&[
            "-c",
            "f() { echo $1 $#; }\nf x\nset -- p q\necho $0 $1 $#",
            "me",
            "y",
        ]),
        0,
        "x 1\nme p 2\n",
    );
}

#[test]
fn exit_and_errexit_stop_the_commands() {
    let sandbox = Sandbox::new("launch stop");

    assert_run(
        &sandbox.run(&["-c", "echo one\nexit 5\necho never"]),
        5,
        "one\n",
    );
    assert_run(
        &sandbox.run(&["-c", "set -e\necho one\nfalse\necho never"]),
        1,
        "one\n",
    );
}

#[test]
fn the_exit_trap_runs_and_only_its_exit_changes_the_status() {
    let sandbox = Sandbox::new("launch trap");

    assert_run(
        &sandbox.run(&["-c", "trap 'echo bye' EXIT\nfalse"]),
        1,
        "bye\n",
    );
    assert_run(&sandbox.run(&["-c", "trap 'exit 7' EXIT\ntrue"]), 7, "");
}

#[test]
fn text_that_cannot_be_understood_stops_with_status_2() {
    let sandbox = Sandbox::new("launch syntax");

    for text in [
        "fi\necho never",
        "echo \"open\necho never\"",
        "if true; then\necho never",
        "cat <<EOF\nnever",
    ] {
        let output = sandbox.run(&["-c", text]);
        assert_run(&output, 2, "");
        assert!(!stderr(&output).is_empty(), "{text}");
    }
}

#[test]
fn a_missing_command_string_is_a_usage_error() {
    let sandbox = Sandbox::new("launch usage");

    let output = sandbox.run(&["-lc"]);

    assert_run(&output, 2, "");
    assert_eq!(
        stderr(&output),
        "terminal_rust: -c: option requires an argument\n"
    );
}

#[test]
fn a_script_gets_its_path_and_arguments() {
    let sandbox = Sandbox::new("launch script");
    // A byte order mark, a `#!` line and Windows line endings.
    sandbox.write(
        "run.trs",
        "\u{feff}#!/usr/bin/env terminal_rust\r\n# comment\r\necho $0 $1 $#\r\n\
         cat <<EOF\r\n# kept\r\nEOF\r\n",
    );

    let output = sandbox.run(&["run.trs", "-c", "b"]);
    assert_run(&output, 0, "run.trs -c 2\n# kept\n");
    // Neither the `#!` line nor the comment ran as a command.
    assert_eq!(stderr(&output), "");
    assert_run(&sandbox.run(&["--", "run.trs"]), 0, "run.trs 0\n# kept\n");
    assert_run(
        &sandbox.run(&["-l", "./run.trs", "a"]),
        0,
        "./run.trs a 1\n# kept\n",
    );
}

#[test]
fn a_script_names_the_line_that_failed_and_goes_on() {
    let sandbox = Sandbox::new("launch script errors");
    sandbox.write(
        "errors.trs",
        "echo a\nterminal-rust-no-such-program\necho b\n",
    );

    let output = sandbox.run(&["errors.trs"]);

    assert_run(&output, 0, "a\nb\n");
    assert_eq!(
        stderr(&output),
        "terminal_rust: errors.trs:2: unknown command: terminal-rust-no-such-program\n"
    );
}

#[test]
fn an_unfinished_script_fails_after_running_what_was_complete() {
    let sandbox = Sandbox::new("launch unfinished");
    sandbox.write("open.trs", "echo a\ncat <<EOF\nnever\n");

    let output = sandbox.run(&["open.trs"]);

    assert_run(&output, 2, "a\n");
    assert!(
        stderr(&output).starts_with("terminal_rust: open.trs:3: "),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_alias_of_a_script_is_silent_and_is_not_kept() {
    let sandbox = Sandbox::new("launch alias");
    sandbox.write("alias.trs", "alias greet echo\ngreet hi\n");

    assert_run(&sandbox.run(&["alias.trs"]), 0, "hi\n");
    assert_run(&sandbox.run(&["-c", "alias greet echo"]), 0, "");

    let listed = sandbox.run(&["alias", "--list"]);
    assert!(!stdout(&listed).contains("greet"), "{}", stdout(&listed));
}

#[track_caller]
fn assert_script_rejected(sandbox: &Sandbox, path: &str, status: i32) {
    let output = sandbox.run(&[path]);
    assert_run(&output, status, "");
    assert!(
        stderr(&output).starts_with(&format!("terminal_rust: {path}: ")),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_script_is_127_and_one_that_cannot_run_is_126() {
    let sandbox = Sandbox::new("launch rejected");
    std::fs::create_dir(sandbox.work().join("folder")).expect("the folder should be created");
    sandbox.write("zero.trs", b"echo never\n\0");
    sandbox.write("latin.trs", b"echo never \xe9\n");
    sandbox.write("huge.trs", "#".repeat(1_048_577));

    assert_script_rejected(&sandbox, "./missing", 127);
    assert_script_rejected(&sandbox, "missing.sh", 127);
    for path in ["./folder", "zero.trs", "latin.trs", "huge.trs"] {
        assert_script_rejected(&sandbox, path, 126);
    }
    #[cfg(unix)]
    assert_script_rejected(&sandbox, "zero.trs/below", 126);
}

#[test]
fn an_operand_is_a_command_unless_a_file_has_its_name() {
    let sandbox = Sandbox::new("launch operand");

    assert_run(&sandbox.run(&["echo", "hi"]), 0, "hi\n");
    assert_run(&sandbox.run(&["--", "echo", "hi"]), 0, "hi\n");

    sandbox.write("echo", "echo from the file $1\n");
    assert_run(&sandbox.run(&["echo", "hi"]), 0, "from the file hi\n");
}

#[test]
fn piped_commands_run_without_a_prompt() {
    let sandbox = Sandbox::new("launch piped");

    let output = sandbox.pipe(&[], "# comment\necho a\nfalse\nif true; then\necho b\nfi\n");
    assert_run(&output, 0, "a\nb\n");
    assert_eq!(stderr(&output), "");

    assert_run(&sandbox.pipe(&[], "echo a\nfalse\n"), 1, "a\n");
    assert_run(&sandbox.pipe(&["-l"], "exit 4\necho never\n"), 4, "");
    assert_run(
        &sandbox.pipe(&[], "read v\nvalue\necho got $v\n"),
        0,
        "got value\n",
    );
}

#[test]
fn only_an_interactive_shell_runs_its_startup() {
    let sandbox = Sandbox::new("launch startup");
    sandbox.write("startup.trs", "echo from-startup\n");
    assert!(
        sandbox
            .run(&["script", "startup", "startup.trs"])
            .status
            .success()
    );
    let run = |args: &[&str]| {
        let mut command = sandbox.shell();
        command
            .args(args)
            .env("TERMINAL_RUST_FASTFETCH", "1")
            .env("TERMINAL_RUST_NATIVE_UI", "1");
        common::finish(command, b"echo x\n")
    };

    assert_run(&run(&[]), 0, "x\n");

    let interactive = run(&["-i"]);
    assert!(
        stdout(&interactive).contains("from-startup"),
        "{}",
        stdout(&interactive)
    );
}

#[cfg(unix)]
#[test]
fn an_argument_that_is_not_utf8_is_refused() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let sandbox = Sandbox::new("launch encoding");
    let mut command = sandbox.shell();
    command.arg("echo").arg(OsStr::from_bytes(b"\xff"));

    let output = common::finish(command, b"");

    assert_run(&output, 2, "");
    assert_eq!(
        stderr(&output),
        "terminal_rust: argument 2 is not valid UTF-8 text\n"
    );
}

#[cfg(unix)]
#[test]
fn a_script_with_a_shebang_line_runs_by_itself() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new("launch shebang");
    let script = sandbox.write("tool", "#!/usr/bin/env terminal_rust\necho ran $1\n");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the script should be made executable");
    let binaries = std::path::Path::new(common::BINARY)
        .parent()
        .expect("the binary has a directory");
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let path = std::env::join_paths(
        std::iter::once(binaries.to_path_buf()).chain(std::env::split_paths(&inherited)),
    )
    .expect("the search path should be valid");

    // Another test starting a program can hold the new file open for an instant.
    let output = loop {
        let mut command = sandbox.command(&script);
        command.arg("well").env("PATH", &path);
        match command.output() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            result => break result.expect("the script should start"),
        }
    };

    assert_run(&output, 0, "ran well\n");
}
