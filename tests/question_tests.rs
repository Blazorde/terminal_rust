//! A question of the shell reaches the person who answers it, wherever the output
//! of the command goes.

// Only part of the shared helpers is used here.
#[allow(dead_code)]
mod common;
#[allow(dead_code)]
#[path = "common/fake_programs.rs"]
mod fake_programs;

use std::fs;

use common::{Sandbox, finish, stderr, stdout};
use fake_programs::Fakes;

/// Run `script`, answer no, and check that the question is on the standard error of
/// the shell and nowhere in what the command redirected.
#[track_caller]
fn assert_asked(sandbox: &Sandbox, fakes: &Fakes, script: &str, question: &str) {
    let target = sandbox.work().join("doomed");
    fs::create_dir_all(&target).expect("fixture");
    fs::write(sandbox.work().join("log"), "").expect("fixture");
    let mut command = fakes.shell(sandbox);
    command.args(["-c", script]);

    let output = finish(command, b"n\n");

    let log = fs::read_to_string(sandbox.work().join("log")).expect("log");
    assert!(
        stderr(&output).contains(question),
        "{script}: stderr {:?}, log {log:?}",
        stderr(&output)
    );
    assert!(!log.contains(question), "{script}: {log:?}");
    assert!(!stdout(&output).contains(question), "{script}");
    assert!(
        target.is_dir(),
        "{script}: a declined removal removes nothing"
    );
}

#[test]
fn the_question_of_rm_is_never_redirected() {
    let sandbox = Sandbox::new("question rm");
    let fakes = Fakes::new(&sandbox);

    for script in [
        "rm -r doomed > log",
        "command rm -r doomed > log",
        "f() { rm -r doomed; }; f > log",
        "f() { rm -r doomed; }; f > log 2>&1",
        "time rm -r doomed > log",
        "eval 'rm -r doomed' > log",
        "{ rm -r doomed; } > log",
        "rm -r doomed 2> log",
        "x=$(rm -r doomed); echo $x > log",
        "rm -r doomed | cat > log",
    ] {
        assert_asked(&sandbox, &fakes, script, "recursively remove directory");
    }
}

#[test]
fn the_question_before_the_system_rm_is_never_redirected() {
    let sandbox = Sandbox::new("question system rm");
    let fakes = Fakes::new(&sandbox);
    fakes.add("rm", 0);

    for script in [
        "rm -rI doomed > log",
        "command rm -rI doomed > log",
        "f() { rm -rI doomed; }; f > log",
    ] {
        assert_asked(&sandbox, &fakes, script, "recursively remove directory");
    }
}

#[test]
fn the_prompt_of_read_is_never_redirected() {
    let sandbox = Sandbox::new("question read");
    let fakes = Fakes::new(&sandbox);

    for script in [
        "read -p 'ASK ' answer > log",
        "f() { read -p 'ASK ' answer; }; f > log 2>&1",
        "{ read -p 'ASK ' answer; } 2> log",
    ] {
        assert_asked(&sandbox, &fakes, script, "ASK ");
    }
}
