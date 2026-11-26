//! The names of the remotes: where they go and what is read from `config`.

use std::fs;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use super::directory_tests::{offered, tab};
use super::remotes::{self, MAX_CONFIG_BYTES, MAX_REMOTES};
use super::test_repository::Sandbox;
use crate::app::history;
use crate::app::parser::parse_words;

const URL: &str = "https://example.invalid/repository.git";

/// A repository with the branch `topic` and the remotes `origin` and `upstream`.
fn with_remotes(sandbox: &Sandbox, relative: &str) -> PathBuf {
    let project = sandbox.repository(relative, &["topic"]);
    for name in ["origin", "upstream"] {
        sandbox.git(&project, &["remote", "add", name, URL]);
    }
    project
}

fn append_config(project: &Path, text: &[u8]) {
    let config = project.join(".git/config");
    let mut bytes = fs::read(&config).expect("config");
    bytes.extend_from_slice(text);
    fs::write(config, bytes).expect("config");
}

#[test]
fn the_first_argument_of_a_command_that_takes_a_repository_is_a_remote() {
    let sandbox = Sandbox::new("remote-first");
    let project = with_remotes(&sandbox, "project");

    for line in [
        "git fetch ",
        "git pull ",
        "git push ",
        "git push -u ",
        "git push --force-with-lease -- ",
        "git fetch --depth=1 ",
        "git fetch -j4 ",
        "git fetch --depth 1 ",
        "git push -o skip ",
        "git push --repo elsewhere ",
        "git push --recurse-submodules check ",
        "git pull -s ours ",
        "git pull --rebase -X theirs ",
        // The value of these is attached or absent.
        "git pull --rebase ",
        "git fetch --recurse-submodules ",
        "git push --signed ",
    ] {
        assert_eq!(
            offered(line, &project, None),
            ["origin ", "upstream "],
            "{line}"
        );
    }
    assert_eq!(offered("git push up", &project, None), ["upstream "]);
}

#[test]
fn the_other_arguments_keep_the_references() {
    let sandbox = Sandbox::new("remote-later");
    let project = with_remotes(&sandbox, "project");

    for line in [
        "git push origin ",
        "git pull origin ",
        "git fetch -p origin ",
        "git push -o skip origin ",
        "git push origin topic ",
        // The value of an option is being typed.
        "git fetch --depth ",
        "git push -o ",
        // These take no repository.
        "git checkout ",
        "git log remote ",
        "git merge --no-ff ",
    ] {
        assert_eq!(offered(line, &project, None), ["main ", "topic "], "{line}");
    }
}

#[test]
fn the_actions_of_git_remote_take_a_remote() {
    let sandbox = Sandbox::new("remote-actions");
    let project = with_remotes(&sandbox, "project");

    for line in [
        "git remote remove ",
        "git remote rename ",
        "git remote show ",
        "git remote prune ",
        "git remote set-url ",
        "git remote get-url ",
        "git remote -v show ",
        "git remote set-url --push ",
        "git remote get-url --push --all ",
    ] {
        assert_eq!(
            offered(line, &project, None),
            ["origin ", "upstream "],
            "{line}"
        );
    }
    for line in [
        "git remote ",
        "git remote -v ",
        "git remote add ",
        "git remote update ",
        "git remote rename origin ",
        "git remote set-url origin ",
        "git remote show origin ",
        "git remote show --",
    ] {
        assert!(offered(line, &project, None).is_empty(), "{line}");
    }
}

#[test]
fn the_remotes_come_from_the_configuration_not_from_the_tracking_branches() {
    let sandbox = Sandbox::new("remote-source");
    let project = sandbox.repository("project", &[]);
    sandbox.git(&project, &["update-ref", "refs/remotes/ghost/main", "HEAD"]);
    // Never fetched: it has no reference.
    sandbox.git(&project, &["remote", "add", "fresh", URL]);

    assert_eq!(remotes::list(&project), ["fresh"]);
    assert_eq!(offered("git push ", &project, None), ["fresh "]);
}

#[test]
fn a_linked_worktree_and_a_directory_option_reach_the_shared_configuration() {
    let sandbox = Sandbox::new("remote-worktree");
    let project = with_remotes(&sandbox, "project");
    sandbox.git(&project, &["worktree", "add", "-q", "../linked"]);
    let linked = sandbox.path().join("linked");
    let deep = sandbox.directory("linked/deep");

    assert_eq!(remotes::list(&linked), ["origin", "upstream"]);
    assert_eq!(remotes::list(&deep), ["origin", "upstream"]);
    for line in ["git -C linked push ", "git -C linked/deep remote show "] {
        assert_eq!(
            offered(line, sandbox.path(), None),
            ["origin ", "upstream "],
            "{line}"
        );
    }
    assert!(offered("git -C $NOWHERE push ", &project, None).is_empty());
}

/// Configurations written by hand, each added to the one `git init` made.
const HAND_WRITTEN: [&str; 24] = [
    "[remote \"origin\"]\n\turl = u\n",
    "[REMOTE \"CaseKept\"]\n URL=u\n[remote \"same\"]\nurl=u\n[remote \"Same\"]\nurl=u\n",
    "[remote \"es\\\"quote\"]\nurl=u\n[remote \"back\\\\slash\"]\nurl=u\n[remote \"o\\ther\"]\nurl=u\n",
    "[remote \"inline\"] url = u\n[core] [remote \"second\"] url = u\n",
    "[remote \"empty\"]\n[remote \"after\"]\n\n# only a comment\n",
    "# [remote \"hash\"]\n; [remote \"semi\"]\n  [remote \"real\"] # tail\n url = u ; [remote \"tail\"]\n",
    "[alias]\n x = a \\\n[remote \"continued\"]\n y = 1\n",
    "[alias]\n x = \"a \\\n[remote \\\"quoted\\\"]\"\n y = 1\n",
    "[alias]\n x = a # no continuation \\\n[remote \"after-comment\"]\n url = u\n",
    "[alias]\n x = \"#\" \\\n[remote \"after-hash\"]\n",
    "[alias]\n x = a\\\\\n[remote \"after-backslash\"]\n url = u\n",
    "[alias]\n x = [remote \"value\"]\n",
    "[remote.dotted]\nurl=u\n[Remote.MiXed]\nurl=u\n[remote.part \"Sub\"]\nurl=u\n",
    "[remote \t \"spaced\"]\nurl=u\n[remote \"with space\"]\nurl=u\n[remote \"br]ket[\"]\nurl=u\n",
    "[remote \"flag\"]\n prune\n[remote \"valueless\"]\n tagopt =\n",
    "[remote \"crlf\"]\r\n url = a \\\r\n[remote \"crlf-continued\"]\r\n",
    "[remote \"/slash\"]\nurl=u\n[remote \"kept/slash\"]\nurl=u\n",
    "[remotes \"plural\"]\nurl=u\n[remote]\n pushDefault = ghost\n[branch \"main\"]\n remote = ghost\n",
    "[url \"https://example.invalid/\"]\n insteadOf = gone:\n[submodule \"remote\"]\nurl=u\n",
    "[remote \"twice\"]\nurl=u\n[core]\nx=1\n[remote \"twice\"]\nfetch=f\n",
    "[remote \"no-newline\"]\n url = u",
    "[remote \"trailing\"]\n url = u \\",
    "[remote \"\u{e9}t\u{e9}\"]\nurl=u\n[remote \"semi;colon#hash\"]\nurl=u\n",
    "[remote \"key-only-at-end\"]\n prune",
];

#[test]
fn the_names_are_those_git_remote_lists() {
    let sandbox = Sandbox::new("remote-differential");
    let project = sandbox.repository("project", &[]);
    let config = project.join(".git/config");
    let initial = fs::read(&config).expect("config");
    let compare = |content: &[u8]| {
        fs::write(&config, content).expect("config");
        let listed = sandbox.git_output(&project, &["remote"]);
        let mut expected: Vec<&str> = listed.lines().collect();
        expected.sort_unstable();
        assert_eq!(
            remotes::list(&project),
            expected,
            "{}",
            String::from_utf8_lossy(content)
        );
    };

    for text in HAND_WRITTEN {
        compare(&[initial.as_slice(), text.as_bytes()].concat());
    }
    compare(b"\xef\xbb\xbf[remote \"marked\"]\nurl=u\n");
    compare(b"[user]\n name = \xff \\\n[remote \"inside\"]\n[remote \"latin\"]\nurl=\xe9\n");

    fs::write(&config, &initial).expect("config");
    for name in [
        "origin",
        "a.b",
        "team/fork",
        "$(x)",
        "semi;colon",
        "wow!",
        "a`b",
    ] {
        sandbox.git(&project, &["remote", "add", name, URL]);
    }
    compare(&fs::read(&config).expect("config"));
    sandbox.git(&project, &["remote", "rename", "origin", "renamed"]);
    compare(&fs::read(&config).expect("config"));
}

#[test]
fn a_line_git_would_reject_ends_the_reading() {
    let sandbox = Sandbox::new("remote-rejected");
    let project = sandbox.repository("project", &[]);

    for rejected in [
        "[remote \"open]\n",
        "[remote\n\"split\"]\n",
        "[remote name]\n",
        "[remote \"tail\" ]\n",
        "[]\n",
        "= value\n",
        "key value\n",
        "x = \"open\n",
        "x = \\q\n",
    ] {
        let config =
            format!("[remote \"before\"]\nurl=u\n[alias]\n{rejected}[remote \"beyond\"]\nurl=u\n");
        fs::write(project.join(".git/config"), config).expect("config");

        assert_eq!(remotes::list(&project), ["before"], "{rejected:?}");
    }
}

#[test]
fn a_huge_configuration_is_read_up_to_the_limit() {
    let sandbox = Sandbox::new("remote-huge");
    let project = sandbox.repository("project", &[]);
    let mut config = String::from("[remote \"first\"]\nurl=u\n#");
    config.push_str(&"x".repeat(MAX_CONFIG_BYTES - config.len()));
    config.push_str("\n[remote \"beyond\"]\nurl=u\n");
    fs::write(project.join(".git/config"), config).expect("config");

    assert_eq!(remotes::list(&project), ["first"]);
}

#[test]
fn the_number_of_names_is_bounded() {
    let sandbox = Sandbox::new("remote-many");
    let project = sandbox.repository("project", &[]);
    let config: String = (0..MAX_REMOTES + 50)
        .map(|number| format!("[remote \"r{number:05}\"]\nurl=u\n"))
        .collect();
    fs::write(project.join(".git/config"), config).expect("config");

    assert_eq!(remotes::list(&project).len(), MAX_REMOTES);
}

#[test]
fn a_repository_without_remote_or_configuration_leaves_the_file_names() {
    let sandbox = Sandbox::new("remote-none");
    let project = sandbox.repository("project", &["topic"]);
    sandbox.file("project/notes.txt");
    let quoted = format!("\"{}\"", project.display());

    assert!(offered("git push ", &project, None).is_empty());
    assert_eq!(tab(&format!("git -C {quoted} push no")), ["notes.txt"]);
    fs::remove_file(project.join(".git/config")).expect("config");
    assert!(remotes::list(&project).is_empty());
    assert!(remotes::list(&sandbox.directory("outside")).is_empty());
}

#[test]
fn remotes_complete_through_the_line_editor() {
    let sandbox = Sandbox::new("remote-editor");
    let project = with_remotes(&sandbox, "with space");
    sandbox.file("with space/notes.txt");
    sandbox.directory("with space/mirror");
    let quoted = format!("\"{}\"", project.display());
    let after = |rest: &str| tab(&format!("git -C {quoted} {rest}"));

    assert_eq!(after("push "), ["origin ", "upstream "]);
    assert_eq!(after("fetch UP"), ["upstream "]);
    assert_eq!(after("remote -v show or"), ["origin "]);
    // A path is a repository too.
    assert_eq!(after("pull no"), ["notes.txt"]);
    assert_eq!(after("pull mir"), [format!("mirror{MAIN_SEPARATOR}")]);
    // The names in the case typed come first, files included.
    assert_eq!(after("push NO"), ["notes.txt"]);
    assert_eq!(after("push origin to"), ["topic "]);
}

#[test]
fn a_hostile_name_is_neither_run_nor_read_as_an_option() {
    let sandbox = Sandbox::new("remote-hostile");
    let project = sandbox.repository("project", &[]);
    append_config(
        &project,
        b"[remote \"-x\"]\nurl=u\n[remote \"--upload-pack=touch\"]\nurl=u\n\
          [remote \"esc\x1b]0;title\x07\"]\nurl=u\n[remote \"re\xe2\x80\xaeversed\"]\nurl=u\n\
          [remote \"$(touch pwned)\"]\nurl=u\n[remote \"a;b\"]\nurl=u\n[remote \"bang!!\"]\nurl=u\n\
          [remote \"`id`\"]\nurl=u\n[remote \"$HOME\"]\nurl=u\n[remote \"o'q\\\" x\"]\nurl=u\n",
    );
    let names = [
        "$(touch pwned)",
        "$HOME",
        "`id`",
        "a;b",
        "bang!!",
        "o'q\" x",
    ];

    assert_eq!(remotes::list(&project), names);
    let replacements = tab(&format!("git -C \"{}\" push ", project.display()));
    let mut read_back: Vec<String> = replacements
        .iter()
        .map(|replacement| {
            assert_eq!(history::expand(replacement, &[]), Ok(None), "{replacement}");
            let words = parse_words(replacement, &|_| Some("EXPANDED".to_string()))
                .expect("the quoting should parse");
            assert_eq!(words.len(), 1, "{replacement}");
            assert!(words[0].pattern.is_none(), "{replacement}");
            words[0].text.clone()
        })
        .collect();
    read_back.sort();
    assert_eq!(read_back, names);
}
