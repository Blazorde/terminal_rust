//! The options offered for a word that starts with `-`.
//!
//! Those of a built-in come from its own definition: the usage line shown by
//! `help`, and the table of what it handles before the system program takes over.
//! Those of Git are not defined here by anything, so the common ones are listed.

use std::collections::BTreeSet;

use rustyline::completion::Pair;

use crate::app::command::command_usage;
use crate::commands::system_fallback;

const GIT_OPTIONS: [(&str, &str); 9] = [
    (
        "branch",
        "-D -a -d -m -r -u -v -vv --all --delete --list --merged --move --no-merged \
         --remotes --set-upstream-to= --show-current --verbose",
    ),
    (
        "checkout",
        "-B -b -f -p -t --detach --force --orphan --ours --patch --theirs --track",
    ),
    (
        "commit",
        "-a -m -p -s -v --all --allow-empty --amend --author= --dry-run --fixup= --message= \
         --no-edit --patch --signoff --verbose",
    ),
    (
        "diff",
        "--cached --check --name-only --name-status --staged --stat --word-diff",
    ),
    (
        "log",
        "-n -p --all --author= --decorate --follow --graph --grep= --oneline --patch \
         --reverse --since= --stat",
    ),
    (
        "pull",
        "-v --autostash --ff-only --no-ff --no-rebase --prune --rebase --tags --verbose",
    ),
    (
        "push",
        "-d -f -n -u -v --all --delete --dry-run --follow-tags --force --force-with-lease \
         --set-upstream --tags --verbose",
    ),
    (
        "stash",
        "-S -a -k -m -p -u --all --include-untracked --keep-index --message= --patch --staged",
    ),
    (
        "switch",
        "-C -c -d -m -t --create --detach --discard-changes --force-create --merge --orphan \
         --track",
    ),
];

/// The options written in a usage line such as `ls [-1a] [--all] [path...]`:
/// each letter of a group is one option.
fn in_usage(usage: &str) -> impl Iterator<Item = String> + '_ {
    usage
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .flat_map(|token| match token.strip_prefix('-') {
            Some(long) if long.starts_with('-') && long.len() > 1 => vec![token.to_string()],
            Some(letters) if !letters.starts_with('-') => {
                letters.chars().map(|letter| format!("-{letter}")).collect()
            }
            _ => Vec::new(),
        })
}

fn of_built_in(name: &str) -> BTreeSet<String> {
    let mut options: BTreeSet<String> =
        command_usage(name).into_iter().flat_map(in_usage).collect();
    if let Some(handled) = system_fallback::handled(name) {
        options.extend(handled.short.chars().map(|letter| format!("-{letter}")));
        options.extend(handled.long.iter().map(|long| long.to_string()));
    }
    options
}

fn of_command(command: &[&str]) -> BTreeSet<String> {
    match command {
        ["git", subcommand, ..] => GIT_OPTIONS
            .iter()
            .filter(|(name, _)| name == subcommand)
            .flat_map(|(_, options)| options.split_whitespace())
            .map(str::to_string)
            .collect(),
        [name, ..] => of_built_in(name),
        [] => BTreeSet::new(),
    }
}

/// The options of `command` that start with `word`. The case always counts:
/// `-r` and `-R` are two options. After `--` there are none.
pub(super) fn complete(command: &[&str], word: &str) -> Option<Vec<Pair>> {
    if !word.starts_with('-') || command.contains(&"--") {
        return None;
    }
    let candidates: Vec<Pair> = of_command(command)
        .into_iter()
        .filter(|option| option.starts_with(word))
        .map(|option| {
            // The value follows a `=` without a space.
            let space = if option.ends_with('=') { "" } else { " " };
            Pair {
                replacement: format!("{option}{space}"),
                display: option,
            }
        })
        .collect();
    (!candidates.is_empty()).then_some(candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offered(command: &[&str], word: &str) -> Vec<String> {
        complete(command, word)
            .unwrap_or_default()
            .into_iter()
            .map(|pair| pair.replacement)
            .collect()
    }

    #[test]
    fn the_options_of_a_usage_line_are_read() {
        let options: Vec<_> =
            in_usage("x [-ab] [+cd] [-o name] [--] [--long] trap - EXIT").collect();

        assert_eq!(options, ["-a", "-b", "-o", "--long"]);
    }

    #[test]
    fn a_built_in_offers_what_its_definition_says() {
        assert_eq!(offered(&["pwd"], "-"), ["-L ", "-P "]);
        assert_eq!(offered(&["read"], "-"), ["-p ", "-r "]);
        assert_eq!(offered(&["echo"], "-"), ["-E ", "-e ", "-n "]);
        assert_eq!(offered(&["unset"], "-"), ["-f "]);
        assert!(offered(&["ls"], "-").contains(&"-R ".to_string()));
        assert!(offered(&["ls"], "--al").contains(&"--all ".to_string()));
        assert_eq!(offered(&["mkdir"], "--p"), ["--parents "]);
        assert_eq!(offered(&["rm"], "--r"), ["--recursive "]);
    }

    /// What the table of the fallback says is handled is offered, all of it.
    #[test]
    fn every_option_handled_by_a_built_in_is_offered() {
        for name in crate::app::command::command_names() {
            let Some(handled) = system_fallback::handled(name) else {
                continue;
            };
            let options = of_built_in(name);
            for letter in handled.short.chars() {
                assert!(options.contains(&format!("-{letter}")), "{name} -{letter}");
            }
            for long in handled.long {
                assert!(options.contains(*long), "{name} {long}");
            }
        }
    }

    #[test]
    fn git_subcommands_have_their_common_options() {
        assert_eq!(offered(&["git", "commit"], "--am"), ["--amend "]);
        assert_eq!(offered(&["git", "commit", "-a"], "--mes"), ["--message="]);
        assert_eq!(
            offered(&["git", "push"], "--force-"),
            ["--force-with-lease "]
        );
        for subcommand in [
            "commit", "checkout", "switch", "push", "pull", "log", "diff", "branch", "stash",
        ] {
            assert!(
                complete(&["git", subcommand], "-").is_some(),
                "{subcommand}"
            );
        }
    }

    #[test]
    fn nothing_is_offered_for_another_word_or_command() {
        assert!(complete(&["ls"], "a").is_none());
        assert!(complete(&["ls", "--"], "-").is_none());
        assert!(complete(&["grep"], "-").is_none());
        assert!(complete(&["git"], "-").is_none());
        assert!(complete(&["git", "frobnicate"], "-").is_none());
        assert!(complete(&[], "-").is_none());
        // The case counts.
        assert!(complete(&["ls"], "--AL").is_none());
    }
}
