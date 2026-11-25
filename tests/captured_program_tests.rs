//! A program started by a command that runs inside the shell writes where the
//! output of that command goes, not on the terminal.

// Only part of the shared helpers is used here.
#[allow(dead_code)]
mod common;
#[path = "common/fake_programs.rs"]
mod fake_programs;

use std::fs;

use common::{Sandbox, assert_run, stderr, stdout};
use fake_programs::Fakes;

fn setup(label: &str) -> (Sandbox, Fakes) {
    let sandbox = Sandbox::new(label);
    let fakes = Fakes::new(&sandbox);
    fakes.add("tool", 0);
    fakes.add("broken", 3);
    (sandbox, fakes)
}

/// The text of a file of the working directory, without the Windows line endings
/// and the blank that `cmd` leaves before them.
fn written(sandbox: &Sandbox, name: &str) -> String {
    fs::read_to_string(sandbox.work().join(name))
        .expect("the output file should exist")
        .lines()
        .map(|line| format!("{}\n", line.trim_end()))
        .collect()
}

#[test]
fn a_redirected_function_takes_the_output_of_its_programs() {
    let (sandbox, fakes) = setup("captured function");

    let output = fakes.run(&sandbox, "f() { tool inside; }; f > out");

    assert_run(&output, 0, "");
    assert_eq!(written(&sandbox, "out"), "tool inside\n");
}

#[test]
fn a_program_run_through_time_or_command_can_be_redirected_and_piped() {
    let (sandbox, fakes) = setup("captured prefixes");

    let output = fakes.run(
        &sandbox,
        "time tool timed > timed; command tool direct > direct; \
         time tool piped | { read line; echo \"got $line\"; }",
    );

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stdout(&output).trim_end(), "got tool piped");
    assert_eq!(written(&sandbox, "timed"), "tool timed\n");
    assert_eq!(written(&sandbox, "direct"), "tool direct\n");
}

#[test]
fn a_process_alias_run_through_time_can_be_redirected() {
    let (sandbox, fakes) = setup("captured alias");

    let output = fakes.run(&sandbox, "alias al tool aliased; time al more > out");

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(written(&sandbox, "out"), "tool aliased more\n");
}

#[test]
fn a_collected_program_keeps_its_status() {
    let (sandbox, fakes) = setup("captured status");

    let output = fakes.run(&sandbox, "command broken > out; echo status=$?");

    assert_eq!(
        stdout(&output).trim_end(),
        "status=3",
        "{}",
        stderr(&output)
    );
    assert_eq!(written(&sandbox, "out"), "broken\n");
}
