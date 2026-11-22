//! Variables exported to the programs the shell starts.
//!
//! The environment of the shell process is never modified: changing it needs
//! `unsafe` code. Exports live in this table instead and are applied to every
//! child process (ADR-0009).

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

#[derive(Clone, Default)]
pub struct Exports {
    set: BTreeMap<String, String>,
    removed: BTreeSet<String>,
}

static EXPORTS: Mutex<Exports> = Mutex::new(Exports {
    set: BTreeMap::new(),
    removed: BTreeSet::new(),
});

fn exports() -> MutexGuard<'static, Exports> {
    // The table stays consistent after a panic, so a poisoned lock is usable.
    EXPORTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn is_valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

pub fn export(name: &str, value: &str) {
    let mut table = exports();
    table.removed.remove(name);
    table.set.insert(name.to_string(), value.to_string());
}

/// The table of exports, to give back with [`restore`] (a subshell, ADR-0027).
pub fn snapshot() -> Exports {
    exports().clone()
}

/// Put back a table taken with [`snapshot`].
pub fn restore(saved: Exports) {
    *exports() = saved;
}

/// Remove `name` from the children's environment, even when it was inherited.
pub fn remove(name: &str) {
    let mut table = exports();
    table.set.remove(name);
    table.removed.insert(name.to_string());
}

/// True when `name` is exported or inherited from the shell's own environment.
pub fn is_exported(name: &str) -> bool {
    value(name).is_some()
}

/// Value of `name` as a child process would see it.
pub fn value(name: &str) -> Option<String> {
    {
        let table = exports();
        if let Some(value) = table.set.get(name) {
            return Some(value.clone());
        }
        if table.removed.contains(name) {
            return None;
        }
    }
    env::var(name).ok()
}

/// The whole environment as a child process would see it.
pub fn vars() -> Vec<(String, String)> {
    let table = exports();
    let mut merged: BTreeMap<String, String> = env::vars()
        .filter(|(name, _)| !table.removed.contains(name))
        .collect();
    merged.extend(table.set.clone());
    merged.into_iter().collect()
}

/// Run `operation` with `vars` exported, then put the previous values back.
/// This is `NAME=value command`: the variable exists for that command only.
pub fn with_temporary<T>(vars: &[(String, String)], operation: impl FnOnce() -> T) -> T {
    let saved: Vec<(String, Option<String>)> = vars
        .iter()
        .map(|(name, _)| (name.clone(), value(name)))
        .collect();
    for (name, new_value) in vars {
        export(name, new_value);
    }
    let result = operation();
    for (name, previous) in saved {
        match previous {
            Some(previous) => export(&name, &previous),
            None => remove(&name),
        }
    }
    result
}

/// Get a child process ready to start: write the output captured so far, so
/// that it stays ahead of what the child prints, then apply the exports.
pub fn prepare_child(command: &mut Command) {
    crate::render::flush_captured();
    let table = exports();
    command.envs(&table.set);
    for name in &table.removed {
        command.env_remove(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn names_follow_the_identifier_rule() {
        for name in ["A", "_x", "PATH", "a1_b2"] {
            assert!(is_valid_name(name), "{name}");
        }
        for name in ["", "1a", "a-b", "a b", "a=b", "é"] {
            assert!(!is_valid_name(name), "{name:?}");
        }
    }

    #[test]
    fn an_exported_value_is_visible_and_listed() {
        export("TRS_TEST_EXPORT_VISIBLE", "one");

        assert_eq!(value("TRS_TEST_EXPORT_VISIBLE"), Some("one".to_string()));
        assert!(is_exported("TRS_TEST_EXPORT_VISIBLE"));
        assert!(vars().contains(&("TRS_TEST_EXPORT_VISIBLE".into(), "one".into())));
    }

    #[test]
    fn exporting_again_replaces_the_value() {
        export("TRS_TEST_EXPORT_REPLACE", "one");
        export("TRS_TEST_EXPORT_REPLACE", "two");

        assert_eq!(value("TRS_TEST_EXPORT_REPLACE"), Some("two".to_string()));
    }

    #[test]
    fn a_removed_variable_hides_an_inherited_one() {
        // Cargo sets this variable for every test process, and no other test
        // reads it, so hiding it cannot disturb tests that run in parallel.
        const INHERITED: &str = "CARGO_MANIFEST_DIR";
        let before = value(INHERITED).expect("cargo should set the manifest directory");
        remove(INHERITED);

        assert_eq!(value(INHERITED), None);
        assert!(!vars().iter().any(|(name, _)| name == INHERITED));

        let mut command = Command::new("terminal-rust-unused");
        prepare_child(&mut command);
        let removed_for_child = command
            .get_envs()
            .any(|(name, value)| name == OsStr::new(INHERITED) && value.is_none());
        assert!(removed_for_child);

        export(INHERITED, &before);
    }

    #[test]
    fn exports_are_applied_to_a_child_command() {
        export("TRS_TEST_EXPORT_CHILD", "child");

        let mut command = Command::new("terminal-rust-unused");
        prepare_child(&mut command);

        assert!(
            command
                .get_envs()
                .any(|(name, value)| name == OsStr::new("TRS_TEST_EXPORT_CHILD")
                    && value == Some(OsStr::new("child")))
        );
    }

    #[test]
    fn exporting_after_a_removal_makes_the_variable_visible_again() {
        remove("TRS_TEST_EXPORT_AGAIN");
        assert_eq!(value("TRS_TEST_EXPORT_AGAIN"), None);

        export("TRS_TEST_EXPORT_AGAIN", "back");

        assert_eq!(value("TRS_TEST_EXPORT_AGAIN"), Some("back".to_string()));
    }
}
