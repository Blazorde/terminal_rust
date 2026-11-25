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

/// A directory holding the shell under the name `name`, to stand for an installed program.
fn installed_program(sandbox: &Sandbox, name: &str) -> std::path::PathBuf {
    let directory = sandbox.home().join("programs");
    std::fs::create_dir_all(&directory).expect("the program directory should be created");
    let program = directory.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    // A link where possible: a file just written can be refused as busy.
    #[cfg(unix)]
    std::os::unix::fs::symlink(common::BINARY, program).expect("the program should be linked");
    #[cfg(not(unix))]
    std::fs::copy(common::BINARY, program).expect("the program should be copied");
    directory
}

#[test]
fn a_built_in_wins_over_a_file_of_its_name() {
    let sandbox = Sandbox::new("launch built-in");
    assert_run(&sandbox.run(&["file-or-command"]), 127, "");
    sandbox.write("file-or-command", "echo from the file\n");
    assert_run(&sandbox.run(&["file-or-command"]), 0, "from the file\n");

    let file = sandbox.write("echo", "echo from the file $1\n");
    // With nothing to search, only the built-in can answer.
    let run = |args: &[&str]| {
        let mut command = sandbox.shell();
        command.args(args).env("PATH", sandbox.home());
        common::finish(command, b"")
    };
    assert_run(&run(&["echo", "hi"]), 0, "hi\n");
    assert_run(&run(&["--", "echo", "hi"]), 0, "hi\n");

    let local = format!(".{}echo", std::path::MAIN_SEPARATOR);
    let absolute = file.to_str().expect("the sandbox path should be text");
    for path in [local.as_str(), absolute] {
        assert_run(&sandbox.run(&[path, "hi"]), 0, "from the file hi\n");
    }
}

#[test]
fn a_program_of_the_search_path_wins_over_a_file_of_its_name() {
    let sandbox = Sandbox::new("launch program");
    let programs = installed_program(&sandbox, "tool");
    sandbox.write("tool", "echo from the file\n");
    let run = |args: &[&str]| {
        let mut command = sandbox.shell();
        command.args(args).env("PATH", &programs);
        common::finish(command, b"")
    };
    let local = format!(".{}tool", std::path::MAIN_SEPARATOR);

    assert_run(&run(&["tool", "echo", "installed"]), 0, "installed\n");
    assert_run(&run(&["--", "tool", "echo", "installed"]), 0, "installed\n");
    assert_run(&run(&[&local]), 0, "from the file\n");
}

// The file in the current directory is both the operand and, through `.`, a program.
#[test]
fn a_relative_entry_placed_first_does_not_take_the_place_of_a_program() {
    let sandbox = Sandbox::new("launch hijack");
    let programs = installed_program(&sandbox, "tool");
    std::fs::create_dir(sandbox.work().join("bin")).expect("the folder should be created");
    let program = format!("tool{}", std::env::consts::EXE_SUFFIX);
    for local in ["tool", &program, &format!("bin/{program}")] {
        let file = sandbox.write(local, "echo HIJACKED\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))
                .expect("the file should be made executable");
        }
        #[cfg(not(unix))]
        let _ = file;
    }
    std::fs::write(
        sandbox.home().join(".terminal_rust_rc"),
        "PATH=.\nexport PATH\n",
    )
    .expect("the rc file should be written");

    for relative in [".", "", "bin"] {
        let path = std::env::join_paths([std::path::Path::new(relative), &programs])
            .expect("the search path should be valid");
        for options in [&[][..], &["-i"][..]] {
            let mut command = sandbox.shell();
            command
                .args(options)
                .args(["tool", "echo", "installed"])
                .env("PATH", &path);

            assert_run(&common::finish(command, b""), 0, "installed\n");
        }
    }

    // A function of the rc file still comes before the program.
    std::fs::write(
        sandbox.home().join(".terminal_rust_rc"),
        "tool() { echo function; }\n",
    )
    .expect("the rc file should be written");
    let mut command = sandbox.shell();
    command.args(["-i", "tool"]).env("PATH", &programs);
    assert_run(&common::finish(command, b""), 0, "function\n");
}

#[cfg(unix)]
#[test]
fn a_login_shell_searches_the_path_of_its_profile() {
    let sandbox = Sandbox::new("launch login program");
    let programs = installed_program(&sandbox, "installed-by-profile");
    std::fs::write(
        sandbox.home().join(".profile"),
        format!("PATH=\"{}:$PATH\"\nexport PATH\n", programs.display()),
    )
    .expect("the profile should be written");
    sandbox.write("installed-by-profile", "echo from the file\n");

    assert_run(
        &sandbox.run(&["-l", "installed-by-profile", "echo", "installed"]),
        0,
        "installed\n",
    );
}

// Otherwise a `#!` script reached through an empty entry would start itself again.
#[cfg(unix)]
#[test]
fn a_program_found_through_a_relative_entry_does_not_count() {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = Sandbox::new("launch relative entry");
    let tool = sandbox.write("tool", "#!/usr/bin/false\necho read as a script\n");
    std::fs::set_permissions(&tool, std::fs::Permissions::from_mode(0o755))
        .expect("the script should be made executable");

    for path in [":/usr/bin", ".:/usr/bin"] {
        let mut command = sandbox.shell();
        command.arg("tool").env("PATH", path);
        let output = common::finish(command, b"");

        assert_run(&output, 0, "read as a script\n");
    }
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

// `sort` is a program of Linux, macOS and Windows that reads all of its input.
#[test]
fn a_program_gets_the_input_that_the_shell_has_not_read() {
    let sandbox = Sandbox::new("launch shared input");

    assert_run(
        &sandbox.pipe(
            &["-c", "read h\necho h=$h\nsort\necho end"],
            "header\nb\na\n",
        ),
        0,
        "h=header\na\nb\nend\n",
    );
    sandbox.write("doomed", "");
    assert_run(
        &sandbox.pipe(&["-c", "rm -i doomed\nsort"], "n\nb\na\n"),
        0,
        "a\nb\n",
    );
}

#[test]
fn a_program_reads_the_rest_of_the_commands_sent_through_a_pipe() {
    let sandbox = Sandbox::new("launch piped commands");

    assert_run(
        &sandbox.pipe(&[], "echo one\nsort\necho two\n"),
        0,
        "one\necho two\n",
    );
}

#[test]
fn an_interactive_shell_leaves_the_input_of_its_commands_in_the_pipe() {
    let sandbox = Sandbox::new("launch interactive input");

    let output = sandbox.pipe(&["-i"], "read h\nheader\necho h=$h\nsort\nb\na\n");

    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(text.contains("h=header\n"), "{text}");
    assert!(text.contains("a\nb\n"), "{text}");
    assert_eq!(stderr(&output), "");
}
