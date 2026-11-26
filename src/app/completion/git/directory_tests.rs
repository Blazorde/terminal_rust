//! `git -C <dir>`: where the arguments are read from.

use std::path::{MAIN_SEPARATOR, Path};
use std::sync::PoisonError;

use rustyline::Context;
use rustyline::completion::Completer;
use rustyline::history::DefaultHistory;

use super::arguments::{has_literal_directory, literal, set_directories_aside};
use super::test_repository::Sandbox;
use super::*;
use crate::app::completion::{ShellHelper, line};

/// The replacements for the last word of `before_cursor`, typed in `current`.
fn offered(before_cursor: &str, current: &Path, home: Option<&Path>) -> Vec<String> {
    let at = line::analyze(before_cursor);
    let folders = Folders { current, home };
    Invocation::read(&at.command, &folders)
        .complete(at.word, Case::Exact)
        .map(|(_, pairs)| pairs.into_iter().map(|pair| pair.replacement).collect())
        .unwrap_or_default()
}

/// What `Tab` at the end of `line` offers through the line editor, typed in the
/// project: no other test changes the current directory meanwhile.
fn tab(line: &str) -> Vec<String> {
    let _guard = crate::test_support::cwd_lock()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let helper = ShellHelper::with_command_names(vec!["git".to_string()]).without_path_programs();
    let history = DefaultHistory::new();
    let (_, pairs) = helper
        .complete(line, line.len(), &Context::new(&history))
        .expect("completion should succeed");
    pairs.into_iter().map(|pair| pair.replacement).collect()
}

#[test]
fn only_the_pairs_before_the_subcommand_are_set_aside() {
    let read = |line: &'static str| {
        let words = set_directories_aside(&line::analyze(line).command);
        (words.command, words.directories, words.directory_next)
    };

    assert_eq!(
        read("git -C a -C 'b c' log "),
        (vec!["git", "log"], vec!["a", "'b c'"], false)
    );
    assert_eq!(read("git -C "), (vec!["git"], vec![], true));
    assert_eq!(read("git -C a -C "), (vec!["git"], vec!["a"], true));
    assert_eq!(read("git -C -C "), (vec!["git"], vec!["-C"], false));
    assert_eq!(
        read("git commit -C "),
        (vec!["git", "commit", "-C"], vec![], false)
    );
    assert_eq!(
        read("git -Ca log "),
        (vec!["git", "-Ca", "log"], vec![], false)
    );
    assert_eq!(read("ls -C a "), (vec!["ls", "-C", "a"], vec![], false));
}

#[test]
fn a_value_is_read_as_the_shell_reads_it() {
    for (typed, value) in [
        ("dir", "dir"),
        ("\"a b\"", "a b"),
        ("'a b'", "a b"),
        ("a\\ b", "a b"),
        ("a\" \"b", "a b"),
        ("'a*b'", "a*b"),
        ("a=b", "a=b"),
        ("~", "/home/me"),
        ("~/x", "/home/me/x"),
        ("\"~\"", "~"),
        ("\"\"", ""),
        ("wow!", "wow!"),
        ("'a!!'", "a!!"),
    ] {
        assert_eq!(
            literal(typed, Some("/home/me")).as_deref(),
            Some(value),
            "{typed}"
        );
    }
}

#[test]
fn a_value_the_shell_would_expand_is_not_read() {
    for typed in [
        "$DIR",
        "a$DIR",
        "\"$DIR\"",
        "$(pwd)",
        "x$(pwd)",
        "`pwd`",
        "\"`pwd`\"",
        "a*",
        "a?",
        "[ab]",
        "{a,b}",
        "a{1..3}",
        "!!",
        "a!x",
        "\"a",
        "'a",
    ] {
        assert_eq!(literal(typed, Some("/home/me")), None, "{typed}");
    }
}

#[test]
fn several_directories_add_up_as_for_git() {
    let sandbox = Sandbox::new("cumulative");
    sandbox.repository("a", &["only-a"]);
    sandbox.repository("a/b", &["only-b"]);
    let other = sandbox.repository("other", &["only-other"]);
    let root = sandbox.path();
    let branches = |directories: &str| {
        offered(
            &format!("git {directories} checkout only-"),
            root,
            Some(root),
        )
    };

    assert_eq!(branches("-C a"), ["only-a "]);
    assert_eq!(branches("-C a -C b"), ["only-b "]);
    assert_eq!(branches("-C a -C \"\""), ["only-a "]);
    assert_eq!(branches("-C '' -C a"), ["only-a "]);
    assert_eq!(branches("-C a/b -C .."), ["only-a "]);
    assert_eq!(
        branches(&format!("-C a -C \"{}\"", other.display())),
        ["only-other "]
    );
    assert_eq!(branches("-C ~/a"), ["only-a "]);
    // Git moves from one to the next: each of them has to exist.
    assert!(branches("-C b").is_empty());
    assert!(branches(&format!("-C nope -C \"{}\"", other.display())).is_empty());
    assert!(branches("-C a -C $NOWHERE").is_empty());
}

#[cfg(unix)]
#[test]
fn the_repository_of_a_link_is_the_one_around_its_target() {
    let sandbox = Sandbox::new("link");
    sandbox.repository("real", &["only-real"]);
    let inside = sandbox.directory("real/inside");
    let elsewhere = sandbox.directory("elsewhere");
    std::os::unix::fs::symlink(&inside, elsewhere.join("link")).expect("link");

    for line in ["git -C link checkout only", "git -C link/.. checkout only"] {
        assert_eq!(offered(line, &elsewhere, None), ["only-real "], "{line}");
    }
}

#[test]
fn the_value_of_the_option_completes_as_a_directory_only() {
    let sandbox = Sandbox::new("value");
    for directory in ["alpha", "a=b", "-dash", ".hidden", "alpha/inner"] {
        sandbox.directory(directory);
    }
    sandbox.file("a-file");
    sandbox.file("alpha/a-file");
    let root = sandbox.path();
    let separator = MAIN_SEPARATOR;

    let mut all = offered("git -C ", root, None);
    all.sort();
    let expected = ["a=b", "alpha", "home"].map(|name| format!("{name}{separator}"));
    assert_eq!(all, expected);
    assert_eq!(
        offered("git -C alpha -C ", root, None),
        [format!("inner{separator}")]
    );
    // A `=` is part of the name, not the start of a path.
    assert_eq!(
        offered("git -C a=", root, None),
        [format!("a=b{separator}")]
    );
    assert!(offered("git -C a-f", root, None).is_empty());
    assert!(offered("git -C -d", root, None).is_empty());
    assert!(offered("git -C $NOWHERE -C ", root, None).is_empty());
    // And the file names do not take over.
    for line in ["git -C ", "git -C a-f"] {
        let at = line::analyze(line);
        let folders = Folders {
            current: root,
            home: None,
        };
        let invocation = Invocation::read(&at.command, &folders);
        assert!(invocation.folders().is_none(), "{line}");
    }
}

#[test]
fn a_value_being_typed_in_a_directory_the_shell_would_expand_offers_nothing() {
    let sandbox = Sandbox::new("typed");
    // Each literal directory exists: taking the word as written would find a child.
    let mut cases = vec![
        ("$NOWHERE", "\"$NOWHERE\"/"),
        ("$NOWHERE", "$NOWHERE/"),
        ("`pwd`", "`pwd`/"),
        ("{a,b}", "{a,b}/"),
        ("{a,b}", "{a,b}/chi"),
        ("{a,b}", "{a,b}/\""),
        ("a!!", "a!!/"),
        ("[ab]", "[ab]/"),
    ];
    // Windows does not allow these characters in a name.
    if cfg!(unix) {
        cases.extend([("x*", "x*/"), ("y?", "y?/"), ("x*", "x*/child/")]);
    }
    let root = sandbox.path();
    for (directory, typed) in cases {
        sandbox.directory(&format!("{directory}/child"));
        sandbox.file(&format!("{directory}/child-file"));

        assert!(
            offered(&format!("git -C {typed}"), root, None).is_empty(),
            "{typed}"
        );
        // Neither do the file names take over.
        let absolute = format!("git -C \"{}\"/{typed}", root.display());
        assert!(tab(&absolute).is_empty(), "{absolute}");
    }
}

#[test]
fn quoted_wildcards_and_special_names_still_complete() {
    let sandbox = Sandbox::new("special");
    sandbox.directory("[ab]/child");
    sandbox.directory("{c,d}");
    sandbox.directory("e!!");
    let root = sandbox.path();
    let separator = MAIN_SEPARATOR;
    let one = |typed: &str| offered(&format!("git -C {typed}"), root, None);

    assert_eq!(one("\"[ab]\"/"), ["[ab]\"/child/"]);
    assert_eq!(one("'[ab]'/"), ["[ab]'/child/"]);
    assert_eq!(one("\"[ab]/"), ["[ab]/child/"]);
    // The name being typed is matched as written, then quoted.
    assert_eq!(one("{c"), [format!("'{{c,d}}'{separator}")]);
    assert_eq!(one("e!"), [format!("'e!!'{separator}")]);
    if cfg!(unix) {
        sandbox.directory("x*/child");
        assert_eq!(one("\"x*\"/"), ["x*\"/child/"]);
        assert_eq!(one("x\\*/"), ["x\\*/child/"]);
        assert_eq!(one("x"), ["x\\*/"]);
    }
}

#[test]
fn a_windows_separator_does_not_escape_the_quote_still_open() {
    for word in ["\"dir\\", "\"a b\\dir\\na", "dir/\"a b", "\"x*\\"] {
        assert!(has_literal_directory(word, None, true), "{word}");
    }
    for word in ["x*\\", "\"$x\\", "\"{a,b}\\na"] {
        assert!(!has_literal_directory(word, None, true), "{word}");
    }
}

#[test]
fn a_redirection_before_the_value_is_not_the_value() {
    let sandbox = Sandbox::new("redirected-value");
    sandbox.directory("only");
    sandbox.file("a-file");
    let root = sandbox.path();
    let expected = [format!("only{MAIN_SEPARATOR}")];

    for line in [
        "git -C >out on",
        "git -C > out on",
        "git -C 2>&1 on",
        "git -C 2>errors on",
    ] {
        assert_eq!(offered(line, root, None), expected, "{line}");
    }
    for line in ["git -C >out a-f", "git -C 2>&1 a-f"] {
        assert!(offered(line, root, None).is_empty(), "{line}");
    }
}

#[test]
fn references_and_options_follow_the_directory_through_the_line_editor() {
    let sandbox = Sandbox::new("editor");
    let spaced = sandbox.repository("with space", &["only-here"]);
    let spaced = format!("\"{}\"", spaced.display());

    assert_eq!(
        tab(&format!("git -C {spaced} checkout ")),
        ["main ", "only-here "]
    );
    assert_eq!(
        tab(&format!("git -C {spaced} checkout ONLY")),
        ["only-here "]
    );
    assert_eq!(
        tab(&format!("sudo git -C {spaced} switch only")),
        ["only-here "]
    );
    assert_eq!(tab(&format!("git -C {spaced} commit --am")), ["--amend "]);
    assert_eq!(tab(&format!("git -C {spaced} sta")), ["stash ", "status "]);
}

#[test]
fn file_names_are_those_of_the_directory() {
    let sandbox = Sandbox::new("files");
    sandbox.repository("repo", &[]);
    sandbox.directory("repo/nested");
    sandbox.file("repo/notes.txt");
    sandbox.file("repo/nested/deep.txt");
    let repository = format!("\"{}\"", sandbox.path().join("repo").display());
    let separator = MAIN_SEPARATOR;

    assert_eq!(
        tab(&format!("git -C {repository} add n")),
        [format!("nested{separator}"), "notes.txt".to_string()]
    );
    assert_eq!(
        tab(&format!("git -C {repository} add nested/d")),
        ["nested/deep.txt"]
    );
    // No reference matches: the files of the directory take over.
    assert_eq!(
        tab(&format!("git -C {repository} checkout note")),
        ["notes.txt"]
    );
}

#[test]
fn a_redirection_stays_in_the_current_directory() {
    let sandbox = Sandbox::new("redirect");
    sandbox.repository("repo", &[]);
    sandbox.file("repo/only-in-repo");
    let repository = format!("\"{}\"", sandbox.path().join("repo").display());

    assert_eq!(
        tab(&format!("git -C {repository} log > Cargo.to")),
        ["Cargo.toml"]
    );
    assert!(tab(&format!("git -C {repository} log > only-in")).is_empty());
    assert_eq!(tab("git -C $NOWHERE log > Cargo.to"), ["Cargo.toml"]);
}

#[test]
fn nothing_is_read_for_a_directory_that_is_not_known() {
    let sandbox = Sandbox::new("unknown");
    let repository = sandbox.repository("repo", &["only-here"]);
    let repository = repository.display();
    let missing = sandbox.path().join("missing");
    // Each of these would lead to a repository if it were taken for granted.
    let unknown = [
        format!("\"{repository}\"$NOWHERE"),
        format!("\"{repository}\"`true`"),
        format!("\"{}\"", missing.display()),
    ];

    // The current directory has this file.
    assert_eq!(tab("git add Cargo.to"), ["Cargo.toml"]);
    for directory in unknown {
        let after = |rest: &str| tab(&format!("git -C {directory} {rest}"));

        assert!(after("checkout ").is_empty(), "{directory}");
        assert!(after("add Cargo.to").is_empty(), "{directory}");
        assert_eq!(after("sta"), ["stash ", "status "], "{directory}");
        assert_eq!(after("commit --am"), ["--amend "], "{directory}");
    }
}
