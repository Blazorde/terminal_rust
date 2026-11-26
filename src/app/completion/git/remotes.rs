//! The remotes declared in the configuration of a repository, read as Git reads
//! that file. The files it includes and the configurations outside the
//! repository are left out.

use std::collections::BTreeSet;
use std::path::Path;

use super::repository::{locate, read_head};
use crate::render::printable::printable;

/// The most bytes read from `config`.
pub(super) const MAX_CONFIG_BYTES: usize = 1024 * 1024;

/// The most remotes offered.
pub(super) const MAX_REMOTES: usize = 1_000;

/// The names of the remotes of the repository that contains `directory`, sorted.
/// Empty outside a repository.
pub(super) fn list(directory: &Path) -> Vec<String> {
    let Some(config) =
        locate(directory).and_then(|common| read_head(&common.join("config"), MAX_CONFIG_BYTES))
    else {
        return Vec::new();
    };
    declared(&config)
        .into_iter()
        .filter_map(|name| String::from_utf8(name).ok())
        // Git would read a name that starts with `-` as an option. One that is not
        // printable could not be inserted, and must leave room to the file names.
        .filter(|name| !name.starts_with('-') && printable(name) == name.as_str())
        .take(MAX_REMOTES)
        .collect()
}

/// The names of the `remote` sections that set a variable, which are those
/// `git remote` lists. Reading stops where Git would reject the file.
///
/// This works on bytes: a line that is not text can still continue on the next
/// one, which must then not be read as a section.
fn declared(text: &[u8]) -> BTreeSet<Vec<u8>> {
    let text = text.strip_prefix(b"\xef\xbb\xbf").unwrap_or(text);
    // Git reads `\r\n` as a single end of line.
    let mut bytes = text
        .iter()
        .enumerate()
        .filter(|&(at, &byte)| !(byte == b'\r' && text.get(at + 1) == Some(&b'\n')))
        .map(|(_, &byte)| byte);
    let mut names = BTreeSet::new();
    // The remote of the current section, until it is known to set something.
    let mut unlisted = None;

    while let Some(byte) = bytes.next() {
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => {}
            b'#' | b';' => {
                bytes.find(|&byte| byte == b'\n');
            }
            b'[' => match section(&mut bytes) {
                Some(section) => unlisted = remote_of(&section),
                None => break,
            },
            _ if byte.is_ascii_alphabetic() => {
                if variable(&mut bytes).is_none() {
                    break;
                }
                names.extend(unlisted.take());
            }
            _ => break,
        }
    }
    names
}

/// The name of the remote a section describes. Git ignores one that starts with
/// `/`.
fn remote_of(section: &[u8]) -> Option<Vec<u8>> {
    let name = section.strip_prefix(b"remote.")?;
    (!name.is_empty() && !name.starts_with(b"/")).then(|| name.to_vec())
}

fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-'
}

/// A section header after its `[`, as Git names it: `a]` or `a.b]` in lower case,
/// and for `a "B"]` the quoted part added as written. `None` when malformed.
fn section(bytes: &mut impl Iterator<Item = u8>) -> Option<Vec<u8>> {
    let mut name = Vec::new();
    let mut byte = bytes.next()?;
    while is_key_byte(byte) || byte == b'.' {
        name.push(byte.to_ascii_lowercase());
        byte = bytes.next()?;
    }
    if byte == b']' {
        return (!name.is_empty()).then_some(name);
    }
    if !matches!(byte, b' ' | b'\t' | b'\r') {
        return None;
    }
    while matches!(byte, b' ' | b'\t' | b'\r') {
        byte = bytes.next()?;
    }
    if byte != b'"' {
        return None;
    }
    name.push(b'.');
    loop {
        match bytes.next()? {
            b'"' => break,
            b'\n' => return None,
            // Any character may be escaped here.
            b'\\' => name.push(bytes.next().filter(|&escaped| escaped != b'\n')?),
            other => name.push(other),
        }
    }
    (bytes.next()? == b']').then_some(name)
}

/// Read past a variable, from the second letter of its key to the end of its
/// value. `None` when malformed.
fn variable(bytes: &mut impl Iterator<Item = u8>) -> Option<()> {
    // The end of the file ends a line.
    let mut next = || bytes.next().unwrap_or(b'\n');
    let mut byte = next();
    while is_key_byte(byte) {
        byte = next();
    }
    while matches!(byte, b' ' | b'\t') {
        byte = next();
    }
    match byte {
        b'\n' => return Some(()),
        b'=' => {}
        _ => return None,
    }
    let (mut quoted, mut comment) = (false, false);
    loop {
        match next() {
            b'\n' => return (!quoted).then_some(()),
            _ if comment => {}
            // Also how a value goes on to the next line.
            b'\\' => {
                if !matches!(next(), b'\n' | b't' | b'b' | b'n' | b'\\' | b'"') {
                    return None;
                }
            }
            b'"' => quoted = !quoted,
            b'#' | b';' if !quoted => comment = true,
            _ => {}
        }
    }
}
