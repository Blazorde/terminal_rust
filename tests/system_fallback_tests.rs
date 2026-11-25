//! A built-in that receives an option it does not handle hands the command to the
//! system program of the same name. The programs here are stand-ins: nothing of
//! the machine runs.

// Only part of the shared helpers is used here.
#[allow(dead_code)]
mod common;
#[path = "common/fake_programs.rs"]
mod fake_programs;

use std::fs;
use std::process::Output;

use common::{Sandbox, finish, stderr, stdout};
use fake_programs::Fakes;

fn setup(label: &str) -> (Sandbox, Fakes) {
    let sandbox = Sandbox::new(label);
    let fakes = Fakes::new(&sandbox);
    for name in ["ls", "date", "touch", "mkdir", "rm"] {
        fakes.add(name, 0);
    }
    // `cat` also copies its input, to show what it was given.
    fakes.script(
        "cat",
        "printf '%s\\n' \"cat $*\"\n\
         while IFS= read -r line; do printf '%s\\n' \"$line\"; done\n",
        "echo cat %*\r\n\"%SystemRoot%\\System32\\findstr.exe\" \"^\"\r\n",
    );
    (sandbox, fakes)
}

/// The lines of a text, without the blank `cmd` leaves after a command with no argument.
fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

#[track_caller]
fn assert_lines(output: &Output, expected: &[&str]) {
    assert_eq!(
        lines(&stdout(output)),
        expected,
        "stderr: {}",
        stderr(output)
    );
}

fn read(sandbox: &Sandbox, name: &str) -> Vec<String> {
    lines(&fs::read_to_string(sandbox.work().join(name)).expect("the file should exist"))
}

/// Run `script` with a `PATH` that holds no program.
fn without_programs(sandbox: &Sandbox, path: &str, script: &str) -> Output {
    let mut command = sandbox.shell();
    command.env("PATH", path).args(["-c", script]);
    finish(command, b"")
}

#[test]
fn the_system_program_gets_every_argument() {
    let (sandbox, fakes) = setup("fallback arguments");

    let output = fakes.run(
        &sandbox,
        "mkdir -m 700 d; touch -d yesterday f; date -u +stamp; cat -n file; ls -a -z dir",
    );

    assert_lines(
        &output,
        &[
            "mkdir -m 700 d",
            "touch -d yesterday f",
            "date -u +stamp",
            "cat -n file",
            "ls -a -z dir",
        ],
    );
    assert_eq!(fs::read_dir(sandbox.work()).expect("listing").count(), 0);
}

#[test]
fn redirections_reach_the_system_program() {
    let (sandbox, fakes) = setup("fallback redirections");

    let output = fakes.run(
        &sandbox,
        "ls -l > out; ls -l again >> out; ls -l errors 2> err >&2; ls -l later > bg & wait",
    );

    assert_lines(&output, &[]);
    assert_eq!(read(&sandbox, "out"), ["ls -l", "ls -l again"]);
    assert_eq!(read(&sandbox, "err"), ["ls -l errors"]);
    assert_eq!(read(&sandbox, "bg"), ["ls -l later"]);
}

#[test]
fn the_output_of_the_system_program_can_be_piped_and_substituted() {
    let (sandbox, fakes) = setup("fallback pipes");

    let output = fakes.run(
        &sandbox,
        "ls -l | { read line; echo \"pipe $line\"; }\n\
         echo \"substitution $(date -u)\"\n\
         alias ll ls -l\n\
         ll dir | { read line; echo \"alias $line\"; }\n\
         command cat -n x | { read line; echo \"command $line\"; }\n\
         time ls -l t | { read line; echo \"time $line\"; }\n\
         command ll c | { read line; echo \"both $line\"; }",
    );

    let shown: Vec<String> = lines(&stdout(&output))
        .into_iter()
        .filter(|line| !line.starts_with("Alias "))
        .collect();
    assert_eq!(
        shown,
        [
            "pipe ls -l",
            "substitution date -u",
            "alias ls -l dir",
            "command cat -n x",
            "time ls -l t",
            "both ls -l c",
        ],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_function_with_a_system_command_can_be_redirected() {
    let (sandbox, fakes) = setup("fallback function");

    let output = fakes.run(&sandbox, "f() { ls -l; cat -n y; }; f > out");

    assert_lines(&output, &[]);
    assert_eq!(read(&sandbox, "out"), ["ls -l", "cat -n y"]);
}

#[test]
fn the_status_of_the_system_program_is_the_status_of_the_command() {
    let (sandbox, fakes) = setup("fallback status");
    fakes.add("touch", 3);

    let output = fakes.run(&sandbox, "touch -c a; echo $?; set -e; touch -c b; echo no");

    assert_lines(&output, &["touch -c a", "3", "touch -c b"]);
    assert_eq!(output.status.code(), Some(3), "{}", stderr(&output));
}

#[test]
fn without_a_system_program_an_unknown_option_creates_nothing() {
    let sandbox = Sandbox::new("fallback missing");

    for (script, message) in [
        ("mkdir -m 700 d", "mkdir: unknown option: -m"),
        ("touch -d yesterday f", "touch: unknown option: -d"),
        ("cat -n f | cat", "cat: unknown option: -n"),
    ] {
        let output = without_programs(&sandbox, "", script);

        assert!(stderr(&output).contains(message), "{}", stderr(&output));
    }
    assert_eq!(fs::read_dir(sandbox.work()).expect("listing").count(), 0);
}

#[test]
fn after_a_double_dash_a_dash_starts_a_file_name() {
    let (sandbox, fakes) = setup("fallback double dash");

    let output = fakes.run(&sandbox, "mkdir -- -x; touch -- -y; mkdir deep/er -p");

    assert_lines(&output, &[]);
    assert!(sandbox.work().join("-x").is_dir());
    assert!(sandbox.work().join("-y").is_file());
    assert!(sandbox.work().join("deep").join("er").is_dir());
    assert!(!sandbox.work().join("--").exists());
}

#[test]
fn a_program_of_a_relative_path_entry_is_never_started() {
    let sandbox = Sandbox::new("fallback relative");
    let planted = if cfg!(windows) { "cat.cmd" } else { "cat" };
    let path = sandbox.write(planted, "#!/bin/sh\necho planted\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    }
    #[cfg(windows)]
    fs::write(&path, "@echo planted\r\n").expect("fixture");

    let output = without_programs(&sandbox, ".", "cat -n f");

    assert_lines(&output, &[]);
    assert!(stderr(&output).contains("cat: unknown option: -n"));
}

#[test]
fn a_function_keeps_its_priority_over_the_system_program() {
    let (sandbox, fakes) = setup("fallback priority");

    let output = fakes.run(
        &sandbox,
        "alias ll ls -l\nls() { echo mine; }\n\
         ls -l\nls -l | cat\nll | cat\ncommand ls -l\ncommand ll",
    );

    let shown: Vec<String> = lines(&stdout(&output))
        .into_iter()
        .filter(|line| !line.starts_with("Alias "))
        .collect();
    // `command` leaves out the function of the name it is given, not the one that an
    // alias reaches.
    assert_eq!(
        shown,
        ["mine", "mine", "mine", "ls -l", "mine"],
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_system_cat_reads_the_input_of_the_block_around_it() {
    let (sandbox, fakes) = setup("fallback input");

    let output = fakes.run(
        &sandbox,
        "echo block | { cat -n; }\nf() { cat -n -; }\necho function | f\ncat -n <<< text",
    );

    assert_lines(
        &output,
        &["cat -n", "block", "cat -n -", "function", "cat -n", "text"],
    );
}

#[test]
fn a_program_that_does_not_read_leaves_the_lines_to_the_loop() {
    let (sandbox, fakes) = setup("fallback loop");
    sandbox.write("lines", "one\ntwo\n");

    let output = fakes.run(&sandbox, "while read x; do ls -l; echo $x; done < lines");

    assert_lines(&output, &["ls -l", "one", "ls -l", "two"]);
}

#[test]
fn the_protections_of_rm_hold_before_the_system_program() {
    let (sandbox, fakes) = setup("fallback rm");
    sandbox.write("kept", "kept");

    for script in [
        "rm -I --no-preserve-root kept",
        "rm -I .",
        "rm -I kept ..",
        "command rm -rI .. | cat",
        "alias del rm -I\ndel . > out",
    ] {
        let output = fakes.run(&sandbox, script);

        assert!(
            !stdout(&output).contains("rm -"),
            "{script}: {}",
            stdout(&output)
        );
        assert!(
            stderr(&output).contains("rm: "),
            "{script}: {}",
            stderr(&output)
        );
    }
}

#[test]
fn the_system_rm_waits_for_the_confirmation_of_a_recursive_removal() {
    let (sandbox, fakes) = setup("fallback rm confirmation");
    fs::create_dir(sandbox.work().join("dir")).expect("fixture");
    let answer = |script: &str, input: &str| {
        let mut command = fakes.shell(&sandbox);
        command.args(["-c", script]);
        stdout(&finish(command, input.as_bytes()))
    };

    for script in ["rm -r -I dir", "rm -r -I dir | cat"] {
        let declined = answer(script, "n\n");
        assert!(declined.contains("recursively remove"), "{declined}");
        assert!(!declined.contains("rm -r -I"), "{declined}");
        let accepted = answer(script, "y\n");
        assert!(accepted.contains("rm -r -I dir"), "{accepted}");
    }
}

#[cfg(target_os = "linux")]
#[test]
fn the_system_program_is_named_after_the_built_in() {
    let sandbox = Sandbox::new("fallback argv0");
    let fakes = Fakes::new(&sandbox);
    // A shell standing for `date` tells the name it was started with.
    let directory = sandbox.home().join("fake programs");
    std::os::unix::fs::symlink("/bin/sh", directory.join("date")).expect("link");

    let output = fakes.run(&sandbox, "date -c 'echo $0'; echo \"$(date -c 'echo $0')\"");

    assert_lines(&output, &["date", "date"]);
}
