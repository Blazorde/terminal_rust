//! What the shell reads before its first command: `~/.terminal_rust_rc` and, for a
//! login shell, the profiles.

mod common;

use std::fs;

use common::{Sandbox, assert_run, stderr, stdout};

const RC_FILE: &str = ".terminal_rust_rc";

fn write_home(sandbox: &Sandbox, name: &str, content: impl AsRef<[u8]>) {
    fs::write(sandbox.home().join(name), content).expect("the home file should be written");
}

#[test]
fn every_interactive_shell_reads_the_rc_file_and_no_other_one() {
    let sandbox = Sandbox::new("startup who");
    write_home(&sandbox, RC_FILE, "echo from-rc\nGREETING=hello\n");
    sandbox.write("show.trs", "echo [$GREETING]\n");

    for (args, input) in [
        (&["-i", "-c", "echo [$GREETING]"][..], ""),
        (&["-i", "show.trs"][..], ""),
        (&["-i", "echo", "[hello]"][..], ""),
        (&["-i"][..], "echo [$GREETING]\n"),
    ] {
        let output = sandbox.pipe(args, input);
        let text = stdout(&output);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(text.starts_with("from-rc\n"), "{args:?}: {text}");
        assert!(text.contains("[hello]"), "{args:?}: {text}");
    }

    assert_run(&sandbox.run(&["-c", "echo [$GREETING]"]), 0, "[]\n");
    assert_run(&sandbox.run(&["show.trs"]), 0, "[]\n");
    assert_run(&sandbox.run(&["echo", "one"]), 0, "one\n");
    assert_run(&sandbox.pipe(&[], "echo [$GREETING]\n"), 0, "[]\n");
}

#[test]
fn what_the_rc_file_sets_up_stays_in_the_shell() {
    let sandbox = Sandbox::new("startup state");
    fs::create_dir(sandbox.home().join("inside")).expect("the directory should be created");
    fs::write(sandbox.home().join("inside").join("marker"), "found\n")
        .expect("the marker should be written");
    write_home(
        &sandbox,
        RC_FILE,
        "\u{feff}# comment\nexport EXPORTED=yes\nalias hi=\"echo hi\"\n\
         greet() {\n  echo function $1\n}\ncd \"$HOME/inside\"\n",
    );

    assert_run(
        &sandbox.run(&["-i", "-c", "echo $EXPORTED\nhi\ngreet x\ncat marker"]),
        0,
        "yes\nhi\nfunction x\nfound\n",
    );
}

#[test]
fn an_alias_typed_after_the_rc_file_still_confirms() {
    let sandbox = Sandbox::new("startup confirm");
    write_home(&sandbox, RC_FILE, "alias quiet=\"echo quiet\"\n");

    let output = sandbox.pipe(&["-i"], "alias loud=\"echo loud\"\n");

    let text = stdout(&output);
    assert!(
        text.contains("Alias 'loud' saved for this session."),
        "{text}"
    );
    assert!(!text.contains("Alias 'quiet'"), "{text}");
}

#[test]
fn an_error_in_the_rc_file_is_located_and_the_rest_still_runs() {
    let sandbox = Sandbox::new("startup error");
    write_home(
        &sandbox,
        RC_FILE,
        "BEFORE=1\nterminal-rust-no-such-program\nAFTER=2\n",
    );

    let output = sandbox.run(&["-i", "-c", "echo $BEFORE $AFTER"]);

    assert_run(&output, 0, "1 2\n");
    let rc = sandbox.home().join(RC_FILE);
    assert_eq!(
        stderr(&output),
        format!(
            "terminal_rust: {}:2: unknown command: terminal-rust-no-such-program\n",
            rc.display()
        )
    );
}

#[test]
fn a_syntax_error_stops_the_rc_file_but_not_the_shell() {
    let sandbox = Sandbox::new("startup syntax");
    for rc in [
        "BEFORE=1\nfi\nAFTER=2\n",
        "BEFORE=1\nif true; then\nAFTER=2\n",
    ] {
        write_home(&sandbox, RC_FILE, rc);

        let output = sandbox.run(&["-i", "-c", "echo [$BEFORE][$AFTER]"]);

        assert_run(&output, 0, "[1][]\n");
        assert!(stderr(&output).contains(RC_FILE), "{}", stderr(&output));
    }
}

#[test]
fn exit_in_the_rc_file_ends_the_shell_with_its_status_and_trap() {
    let sandbox = Sandbox::new("startup exit");
    write_home(
        &sandbox,
        RC_FILE,
        "trap 'echo bye' EXIT\nexit 6\necho never\n",
    );

    for args in [
        &["-i"][..],
        &["-i", "-c", "echo never"],
        &["-i", "echo", "never"],
    ] {
        assert_run(&sandbox.pipe(args, "echo never\n"), 6, "bye\n");
    }
}

#[test]
fn a_trap_of_the_rc_file_runs_after_a_single_command() {
    let sandbox = Sandbox::new("startup trap");
    write_home(&sandbox, RC_FILE, "trap 'echo bye' EXIT\n");

    assert_run(&sandbox.run(&["-i", "echo", "one"]), 0, "one\nbye\n");
}

#[test]
fn an_rc_file_that_cannot_be_read_gives_one_warning() {
    let sandbox = Sandbox::new("startup unreadable");
    let quiet = sandbox.run(&["-i", "-c", "echo ok"]);
    assert_run(&quiet, 0, "ok\n");
    assert_eq!(stderr(&quiet), "", "a missing file says nothing");

    for (content, reason) in [
        (b"echo \0binary\n".to_vec(), "is not a text file"),
        (vec![0xff, 0xfe, b'\n'], "is not a text file"),
        (vec![b'#'; 1_048_577], "is larger than 1 MiB"),
    ] {
        write_home(&sandbox, RC_FILE, content);

        let output = sandbox.run(&["-i", "-c", "echo ok"]);

        assert_run(&output, 0, "ok\n");
        let warning = stderr(&output);
        assert!(warning.contains(reason), "{warning}");
        assert_eq!(warning.lines().count(), 1, "{warning}");
    }

    let rc = sandbox.home().join(RC_FILE);
    fs::remove_file(&rc).expect("the rc file should be removed");
    fs::create_dir(&rc).expect("the directory should be created");
    let output = sandbox.run(&["-i", "-c", "echo ok"]);
    assert_run(&output, 0, "ok\n");
    assert!(stderr(&output).contains("is a directory"));
}

#[cfg(not(unix))]
#[test]
fn the_login_option_changes_nothing_without_unix_profiles() {
    let sandbox = Sandbox::new("startup no login");
    write_home(&sandbox, ".profile", "export TRS_PROFILE=read\n");

    for args in [
        &["-l", "-c", "echo [$TRS_PROFILE]"][..],
        &["--login", "-c", "echo [$TRS_PROFILE]"],
    ] {
        let output = sandbox.run(args);
        assert_run(&output, 0, "[]\n");
        assert_eq!(stderr(&output), "");
    }
}

#[cfg(unix)]
mod login {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    use super::common::{self, Sandbox, assert_run, stdout};
    use super::{RC_FILE, fs, write_home};

    const SHOW: &str = "echo [$TRS_PROFILE]";

    fn profile(sandbox: &Sandbox, content: &str) {
        write_home(sandbox, ".profile", content);
    }

    #[test]
    fn only_a_login_shell_takes_the_variables_of_the_profile() {
        let sandbox = Sandbox::new("login who");
        profile(&sandbox, "export TRS_PROFILE=read\nTRS_PRIVATE=hidden\n");
        sandbox.write("show.trs", SHOW);

        for args in [
            &["-l", "-c", SHOW][..],
            &["-lc", SHOW],
            &["--login", "show.trs"],
            &["-l", "echo", "[read]"],
        ] {
            assert_run(&sandbox.run(args), 0, "[read]\n");
        }
        assert_run(&sandbox.pipe(&["-l"], SHOW), 0, "[read]\n");
        assert!(stdout(&sandbox.pipe(&["-li"], SHOW)).contains("[read]"));
        assert_run(&sandbox.run(&["-c", SHOW]), 0, "[]\n");
        assert_run(
            &sandbox.run(&["-l", "-c", "echo [$TRS_PRIVATE]"]),
            0,
            "[]\n",
        );
    }

    #[test]
    fn a_name_that_starts_with_a_dash_makes_a_login_shell() {
        let sandbox = Sandbox::new("login argv0");
        profile(&sandbox, "export TRS_PROFILE=read\n");
        let mut command = sandbox.shell();
        command.arg0("-terminal_rust").args(["-c", SHOW]);

        assert_run(&common::finish(command, b""), 0, "[read]\n");
    }

    #[test]
    fn a_relative_home_does_not_choose_the_profile() {
        let sandbox = Sandbox::new("login relative");
        let home = sandbox.work().join("relative home");
        fs::create_dir(&home).expect("the directory should be created");
        fs::write(home.join(".profile"), "export TRS_PROFILE=read\n")
            .expect("the profile should be written");
        let mut command = sandbox.shell();
        command
            .env("HOME", "relative home")
            .args(["-l", "-c", SHOW]);

        assert_run(&common::finish(command, b""), 0, "[]\n");
    }

    #[test]
    fn the_rc_file_comes_after_the_profile() {
        let sandbox = Sandbox::new("login order");
        profile(&sandbox, "export TRS_PROFILE=read\n");
        write_home(&sandbox, RC_FILE, "echo rc saw $TRS_PROFILE\n");

        assert_run(
            &sandbox.run(&["-li", "-c", "echo done"]),
            0,
            "rc saw read\ndone\n",
        );
    }

    #[test]
    fn values_come_back_whole_and_what_the_profile_prints_stays_off_the_output() {
        let sandbox = Sandbox::new("login values");
        profile(
            &sandbox,
            "echo noise from the profile\nexport TRS_PROFILE=\"a=b\nc d\"\n",
        );

        let output = sandbox.run(&["-l", "-c", "echo \"$TRS_PROFILE\""]);

        assert_run(&output, 0, "a=b\nc d\n");
        assert!(super::stderr(&output).contains("noise from the profile"));
    }

    #[test]
    fn a_profile_that_exits_or_fails_still_hands_over_its_variables() {
        let sandbox = Sandbox::new("login abort");
        for end in ["exit 3", "set -e\nfalse", "if then", "set -u\n: $TRS_UNSET"] {
            profile(&sandbox, &format!("export TRS_PROFILE=read\n{end}\n"));

            assert_run(&sandbox.run(&["-l", "-c", SHOW]), 0, "[read]\n");
        }
    }

    #[test]
    fn a_program_left_running_by_the_profile_does_not_hold_the_shell() {
        let sandbox = Sandbox::new("login background");
        // It keeps every descriptor it inherits, but the standard error of the test.
        profile(
            &sandbox,
            "export TRS_PROFILE=read\nsleep 30 2>/dev/null >&2 &\necho $! > \"$HOME/pid\"\n",
        );
        let started = Instant::now();

        let output = sandbox.run(&["-l", "-c", SHOW]);

        let elapsed = started.elapsed();
        let pid = fs::read_to_string(sandbox.home().join("pid")).expect("the pid is written");
        sandbox
            .command("/bin/sh")
            .args(["-c", &format!("kill {}", pid.trim())])
            .stderr(Stdio::null())
            .status()
            .expect("the program should be stopped");
        assert_run(&output, 0, "[read]\n");
        assert!(elapsed < Duration::from_secs(20), "{elapsed:?}");
    }

    #[test]
    fn the_profile_cannot_reach_the_descriptor_of_the_environment() {
        let sandbox = Sandbox::new("login descriptor");
        profile(
            &sandbox,
            "if (: >&3) 2>/dev/null; then TRS_PROFILE=open; else TRS_PROFILE=closed; fi\n\
             export TRS_PROFILE\n",
        );

        assert_run(&sandbox.run(&["-l", "-c", SHOW]), 0, "[closed]\n");
    }

    #[test]
    fn programs_are_found_in_the_path_of_the_profile() {
        let sandbox = Sandbox::new("login path");
        let bin = sandbox.home().join("profile bin");
        fs::create_dir(&bin).expect("the directory should be created");
        let program = bin.join("trs-profile-tool");
        fs::write(&program, "#!/bin/sh\necho tool ran\n").expect("the tool should be written");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the tool should be executable");
        profile(&sandbox, "PATH=\"$HOME/profile bin:$PATH\"\nexport PATH\n");

        assert_run(
            &sandbox.run(&["-l", "-c", "trs-profile-tool\necho $(trs-profile-tool)"]),
            0,
            "tool ran\ntool ran\n",
        );
        assert_run(
            &sandbox.run(&["-l", "-c", "command -v trs-profile-tool"]),
            0,
            &format!("{}\n", program.display()),
        );
    }

    #[test]
    fn a_profile_that_leaves_nothing_usable_gives_a_warning_and_a_working_shell() {
        let sandbox = Sandbox::new("login nothing");
        profile(&sandbox, "export TRS_PROFILE=read\nexec true\n");

        let output = sandbox.run(&["-l", "-c", SHOW]);

        assert_run(&output, 0, "[]\n");
        assert!(
            super::stderr(&output).contains("cannot read the login profile"),
            "{}",
            super::stderr(&output)
        );
    }
}
