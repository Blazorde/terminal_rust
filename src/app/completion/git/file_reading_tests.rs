//! Reading the files of a repository: `Tab` must neither block nor fill the
//! memory, whatever stands where a file is expected.

use std::fs;

use super::references::{self, MAX_PACKED_BYTES};
use super::repository::{locate, read_head, text_lines};
use super::test_repository::Sandbox;

const SHA: &str = "1111111111111111111111111111111111111111";

#[test]
fn a_longer_file_loses_the_line_cut_by_the_limit() {
    let sandbox = Sandbox::new("head");
    let file = sandbox.path().join("file");
    fs::write(&file, "one\ntwo\nthree").expect("fixture");
    let head = |limit| read_head(&file, limit).expect("regular file");

    assert_eq!(head(64), b"one\ntwo\nthree");
    assert_eq!(head(13), b"one\ntwo\nthree");
    assert_eq!(head(12), b"one\ntwo\n");
    assert_eq!(head(8), b"one\ntwo\n");
    assert_eq!(head(7), b"one\n");
    assert_eq!(head(3), b"");
    assert_eq!(head(0), b"");
}

#[test]
fn only_a_regular_file_is_read() {
    let sandbox = Sandbox::new("kind");

    assert_eq!(read_head(sandbox.path(), 16), None);
    assert_eq!(read_head(&sandbox.path().join("missing"), 16), None);
}

#[test]
fn lines_that_are_not_text_are_skipped_whole() {
    let lines: Vec<&str> = text_lines(b"one\r\nb\xffd\ntwo\n\nlast").collect();

    assert_eq!(lines, ["one", "two", "", "last"]);
}

#[test]
fn a_packed_name_that_is_not_text_keeps_the_others() {
    let sandbox = Sandbox::new("encoding");
    let project = sandbox.repository("project", &["before"]);
    sandbox.git(&project, &["pack-refs", "--all"]);
    let packed = project.join(".git/packed-refs");
    let mut bytes = fs::read(&packed).expect("packed-refs");
    bytes.extend_from_slice(format!("{SHA} refs/heads/bad-").as_bytes());
    bytes.extend_from_slice(b"\xff\n");
    bytes.extend_from_slice(format!("{SHA} refs/heads/zz-after\n").as_bytes());
    fs::write(&packed, bytes).expect("packed-refs");

    assert_eq!(references::list(&project), ["before", "main", "zz-after"]);
}

#[test]
fn a_huge_packed_file_is_read_up_to_its_last_whole_line() {
    let sandbox = Sandbox::new("huge");
    let project = sandbox.repository("project", &[]);
    sandbox.git(&project, &["pack-refs", "--all"]);
    let packed = project.join(".git/packed-refs");
    let mut text = fs::read_to_string(&packed).expect("packed-refs");
    // The limit falls inside the name `straddling`.
    let cut_line = format!("{SHA} refs/heads/strad");
    let padding = MAX_PACKED_BYTES - cut_line.len() - text.len();
    text.push('#');
    text.push_str(&"x".repeat(padding - 2));
    text.push('\n');
    text.push_str(&format!("{cut_line}dling\n{SHA} refs/heads/beyond\n"));
    fs::write(&packed, text).expect("packed-refs");

    assert_eq!(references::list(&project), ["main"]);
}

#[test]
fn an_oversized_pointer_file_is_not_followed() {
    let sandbox = Sandbox::new("pointer");
    let project = sandbox.repository("project", &[]);
    let linked = sandbox.path().join("linked");
    sandbox.git(&project, &["worktree", "add", "-q", "../linked"]);
    let admin = project.join(".git/worktrees/linked");
    let padding = " ".repeat(64 * 1024);

    fs::write(admin.join("commondir"), format!("../..{padding}")).expect("fixture");
    assert_eq!(references::list(&linked), Vec::<String>::new());

    let dot_git = linked.join(".git");
    let pointer = fs::read_to_string(&dot_git).expect("pointer");
    fs::write(&dot_git, format!("{}{padding}", pointer.trim_end())).expect("fixture");
    assert_eq!(locate(&linked), None);
}

#[cfg(unix)]
mod special_files {
    use std::os::unix::fs::symlink;
    use std::path::Path;
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    fn fifo(path: &Path) {
        let status = Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("mkfifo should be installed");
        assert!(status.success(), "mkfifo {}", path.display());
    }

    /// The references of `directory`, failing instead of waiting for ever.
    fn listed_in_time(directory: &Path) -> Vec<String> {
        let (sender, receiver) = mpsc::channel();
        let directory = directory.to_path_buf();
        thread::spawn(move || sender.send(references::list(&directory)));
        receiver
            .recv_timeout(Duration::from_secs(20))
            .expect("listing the references should not block")
    }

    #[test]
    fn a_fifo_instead_of_packed_refs_does_not_block() {
        let sandbox = Sandbox::new("fifo-packed");
        let project = sandbox.repository("project", &[]);
        fifo(&project.join(".git/packed-refs"));

        assert_eq!(listed_in_time(&project), ["main"]);
    }

    #[test]
    fn a_fifo_instead_of_commondir_does_not_block() {
        let sandbox = Sandbox::new("fifo-commondir");
        let project = sandbox.repository("project", &[]);
        sandbox.git(&project, &["worktree", "add", "-q", "../linked"]);
        let commondir = project.join(".git/worktrees/linked/commondir");
        fs::remove_file(&commondir).expect("fixture");
        fifo(&commondir);

        let names = listed_in_time(&sandbox.path().join("linked"));

        assert_eq!(names, Vec::<String>::new());
    }

    #[test]
    fn a_link_to_an_endless_device_is_not_read() {
        let sandbox = Sandbox::new("endless");
        let project = sandbox.repository("project", &[]);
        let packed = project.join(".git/packed-refs");
        symlink("/dev/zero", &packed).expect("link");

        assert_eq!(read_head(&packed, 16), None);
        assert_eq!(references::list(&project), ["main"]);
    }

    #[test]
    fn a_link_to_a_regular_file_is_followed() {
        let sandbox = Sandbox::new("linked-packed");
        let project = sandbox.repository("project", &["packed"]);
        sandbox.git(&project, &["pack-refs", "--all"]);
        let packed = project.join(".git/packed-refs");
        let moved = sandbox.path().join("moved");
        fs::rename(&packed, &moved).expect("fixture");
        symlink(&moved, &packed).expect("link");

        assert_eq!(references::list(&project), ["main", "packed"]);
    }
}
