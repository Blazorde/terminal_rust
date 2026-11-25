//! A built-in that receives something it does not handle gives the whole command
//! to the system program of the same name. Without one, an unknown option is an
//! error: it is never read as a file name.

use std::path::PathBuf;

/// The operands a built-in handles.
#[derive(Clone, Copy, Debug)]
pub enum Operands {
    Any,
    AtMost(usize),
    None,
    /// A single `+FORMAT`.
    Format,
}

/// The unhandled options that may reach the system program.
#[derive(Clone, Copy, Debug)]
pub enum HandOver {
    Any,
    /// Exact long options, and the letters allowed in a group of short ones.
    Only(&'static [&'static str]),
}

/// What a built-in handles by itself.
#[derive(Debug)]
pub struct Handled {
    pub short: &'static str,
    pub long: &'static [&'static str],
    pub operands: Operands,
    /// Short options the built-in accepts but the system program does better.
    pub prefer_system: &'static str,
    pub hand_over: HandOver,
    /// Whether the program reads its input when it has no file operand.
    pub reads_input: bool,
    /// Whether a program found on Windows may take over.
    pub on_windows: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Decision {
    Builtin,
    System(PathBuf),
    Reject(String),
}

const NOTHING: Handled = Handled {
    short: "",
    long: &[],
    operands: Operands::Any,
    prefer_system: "",
    hand_over: HandOver::Any,
    reads_input: false,
    on_windows: true,
};

const LS: Handled = Handled {
    short: "1aAdFhlpRrSt",
    long: &[
        "--all",
        "--almost-all",
        "--directory",
        "--classify",
        "--human-readable",
        // Not known to GNU `ls`: the built-in keeps it.
        "--long",
        "--recursive",
        "--reverse",
        "--indicator-style=slash",
        "--color",
        "--color=always",
        "--color=yes",
        "--color=force",
        "--color=auto",
        "--color=tty",
        "--color=if-tty",
        "--color=never",
        "--color=no",
        "--color=none",
        "--sort=name",
        "--sort=size",
        "--sort=time",
    ],
    // The built-in long format has no permissions, owner or date.
    prefer_system: "l",
    ..NOTHING
};

const CAT: Handled = Handled {
    reads_input: true,
    ..NOTHING
};

const MKDIR: Handled = Handled {
    short: "p",
    long: &["--parents"],
    ..NOTHING
};

const DATE: Handled = Handled {
    operands: Operands::Format,
    ..NOTHING
};

const PRINTENV: Handled = Handled {
    operands: Operands::AtMost(1),
    ..NOTHING
};

const NO_ARGUMENT: Handled = Handled {
    operands: Operands::None,
    ..NOTHING
};

// An option the built-in does not know could lift one of its protections, such as
// `--no-preserve-root` or an abbreviation: only the listed ones are handed over.
const RM: Handled = Handled {
    short: "dfirRv",
    long: &[
        "--dir",
        "--force",
        "--interactive",
        "--recursive",
        "--verbose",
    ],
    hand_over: HandOver::Only(&[
        "I",
        "--one-file-system",
        "--preserve-root",
        "--preserve-root=all",
        "--interactive=never",
        "--interactive=once",
        "--interactive=always",
        "--help",
        "--version",
    ]),
    // The `rm` of Git or MSYS expands `*` and `@file` in its arguments again, so
    // it could remove more than what the shell has checked.
    on_windows: false,
    ..NOTHING
};

/// What the built-in `name` handles, for those that can hand a command over.
pub fn handled(name: &str) -> Option<&'static Handled> {
    match name {
        "ls" => Some(&LS),
        "cat" => Some(&CAT),
        "mkdir" => Some(&MKDIR),
        "touch" => Some(&NOTHING),
        "date" => Some(&DATE),
        "printenv" => Some(&PRINTENV),
        "whoami" | "arch" | "nproc" | "uptime" | "hostname" | "clear" => Some(&NO_ARGUMENT),
        "rm" => Some(&RM),
        _ => None,
    }
}

/// Who runs `name args` on Windows or elsewhere. `locate` finds the system
/// program of a name.
pub fn decide(
    name: &str,
    args: &[String],
    windows: bool,
    locate: impl Fn(&str) -> Option<PathBuf>,
) -> Decision {
    let Some(handled) = handled(name) else {
        return Decision::Builtin;
    };
    let locate = |name: &str| {
        (handled.on_windows || !windows)
            .then(|| locate(name))
            .flatten()
    };
    let unknown: Vec<&String> = options(args)
        .filter(|option| !handled.knows(option))
        .collect();
    let extra_operands = !handled.accepts(args);

    if unknown.is_empty() && !extra_operands {
        let preferred = options(args).any(|option| {
            short_group(option)
                .is_some_and(|letters| letters.chars().any(|c| handled.prefer_system.contains(c)))
        });
        return match preferred.then(|| locate(name)).flatten() {
            Some(program) => Decision::System(program),
            None => Decision::Builtin,
        };
    }

    let refused = unknown.iter().find(|option| !handled.may_hand_over(option));
    if refused.is_none()
        && let Some(program) = locate(name)
    {
        return Decision::System(program);
    }
    match refused.or(unknown.first()) {
        Some(option) => Decision::Reject(format!("{name}: unknown option: {option}")),
        None => Decision::Builtin,
    }
}

/// The arguments that are not options. The first `--` ends the options and is
/// dropped; `-` alone is an operand.
pub fn operands(args: &[String]) -> impl Iterator<Item = &String> {
    let mut ended = false;
    args.iter().filter(move |arg| {
        if ended {
            return true;
        }
        if *arg == "--" {
            ended = true;
            return false;
        }
        !is_option(arg)
    })
}

/// The options, which all come before the first `--`.
pub fn options(args: &[String]) -> impl Iterator<Item = &String> {
    args.iter()
        .take_while(|arg| *arg != "--")
        .filter(|arg| is_option(arg))
}

fn is_option(arg: &str) -> bool {
    arg.len() > 1 && arg.starts_with('-')
}

/// The letters of `-abc`; `None` for a long option.
pub fn short_group(option: &str) -> Option<&str> {
    option
        .strip_prefix('-')
        .filter(|letters| !letters.starts_with('-'))
}

impl Handled {
    fn knows(&self, option: &str) -> bool {
        match short_group(option) {
            Some(letters) => letters.chars().all(|c| self.short.contains(c)),
            None => self.long.contains(&option),
        }
    }

    fn may_hand_over(&self, option: &str) -> bool {
        let HandOver::Only(allowed) = self.hand_over else {
            return true;
        };
        match short_group(option) {
            Some(letters) => letters.chars().all(|letter| {
                self.short.contains(letter) || allowed.iter().any(|one| one.chars().eq([letter]))
            }),
            None => allowed.contains(&option),
        }
    }

    fn accepts(&self, args: &[String]) -> bool {
        let mut operands = operands(args);
        match self.operands {
            Operands::Any => true,
            Operands::AtMost(count) => operands.count() <= count,
            Operands::None => operands.next().is_none(),
            Operands::Format => {
                operands.next().is_none_or(|first| first.starts_with('+'))
                    && operands.next().is_none()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(ToString::to_string).collect()
    }

    fn found(name: &str) -> Option<PathBuf> {
        Some(PathBuf::from("system").join(name))
    }

    fn missing(_name: &str) -> Option<PathBuf> {
        None
    }

    fn with_program(name: &str, args: &[&str]) -> Decision {
        decide(name, &strings(args), false, found)
    }

    fn without_program(name: &str, args: &[&str]) -> Decision {
        decide(name, &strings(args), false, missing)
    }

    fn system(name: &str) -> Decision {
        Decision::System(PathBuf::from("system").join(name))
    }

    fn reject(message: &str) -> Decision {
        Decision::Reject(message.to_string())
    }

    #[test]
    fn handled_arguments_stay_with_the_built_in() {
        for (name, args) in [
            ("ls", &["-aF", "--color=auto", "dir"][..]),
            ("ls", &["--long"]),
            ("cat", &["file", "-"]),
            ("mkdir", &["a", "-p", "--parents"]),
            ("touch", &["--", "-d"]),
            ("date", &["+%Y"]),
            ("date", &[]),
            ("printenv", &["--", "HOME"]),
            ("hostname", &[]),
            ("rm", &["-rfv", "--dir", "x"]),
            ("echo", &["-n"]),
            ("cd", &["-"]),
        ] {
            assert_eq!(
                with_program(name, args),
                Decision::Builtin,
                "{name} {args:?}"
            );
        }
    }

    #[test]
    fn anything_else_goes_to_the_system_program() {
        for (name, args) in [
            ("cat", &["-n", "file"][..]),
            ("ls", &["-az"]),
            ("ls", &["--sort=extension"]),
            ("mkdir", &["-m", "700", "d"]),
            ("mkdir", &["-pv", "d"]),
            ("touch", &["-d", "yesterday", "f"]),
            ("date", &["-u", "+%s"]),
            ("date", &["+%s", "+%s"]),
            ("date", &["0101"]),
            ("printenv", &["A", "B"]),
            ("hostname", &["-I"]),
            ("nproc", &["--all"]),
            ("whoami", &["x"]),
            ("rm", &["-I", "x"]),
            ("rm", &["-rI", "x"]),
            ("rm", &["--one-file-system", "x"]),
            ("rm", &["--interactive=once", "x"]),
        ] {
            assert_eq!(with_program(name, args), system(name), "{name} {args:?}");
        }
    }

    #[test]
    fn the_long_listing_prefers_the_system_program_and_falls_back() {
        for args in [&["-l"][..], &["-alh", "dir"]] {
            assert_eq!(with_program("ls", args), system("ls"), "{args:?}");
            assert_eq!(without_program("ls", args), Decision::Builtin, "{args:?}");
        }
        assert_eq!(with_program("ls", &["--", "-l"]), Decision::Builtin);
    }

    #[test]
    fn without_a_program_an_unknown_option_is_an_error() {
        assert_eq!(
            without_program("mkdir", &["-p", "-m", "700"]),
            reject("mkdir: unknown option: -m")
        );
        assert_eq!(
            without_program("cat", &["file", "-n"]),
            reject("cat: unknown option: -n")
        );
        assert_eq!(
            without_program("rm", &["-I", "x"]),
            reject("rm: unknown option: -I")
        );
    }

    #[test]
    fn without_a_program_extra_operands_stay_with_the_built_in() {
        assert_eq!(without_program("hostname", &["x"]), Decision::Builtin);
        assert_eq!(without_program("printenv", &["A", "B"]), Decision::Builtin);
    }

    #[test]
    fn rm_hands_over_only_the_listed_options() {
        for option in [
            "--no-preserve-root",
            "--recu",
            "--interactive=yes",
            "-x",
            "-rIx",
        ] {
            assert_eq!(
                with_program("rm", &["-I", option, "x"]),
                reject(&format!("rm: unknown option: {option}")),
                "{option}"
            );
        }
    }

    #[test]
    fn rm_is_never_handed_over_on_windows() {
        for option in [
            "-I",
            "-rI",
            "--one-file-system",
            "--interactive=always",
            "--help",
        ] {
            let args = strings(&[option, "*"]);
            assert_eq!(
                decide("rm", &args, true, found),
                reject(&format!("rm: unknown option: {option}")),
                "{option}"
            );
            assert_eq!(decide("rm", &args, false, found), system("rm"), "{option}");
        }
        assert_eq!(
            decide("rm", &strings(&["-rf", "x"]), true, found),
            Decision::Builtin
        );
        assert_eq!(decide("cat", &strings(&["-n"]), true, found), system("cat"));
    }

    #[test]
    fn operands_drop_the_options_and_the_first_double_dash() {
        let args = strings(&["-p", "a", "-", "--", "-b", "--"]);
        let found: Vec<&String> = operands(&args).collect();

        assert_eq!(found, ["a", "-", "-b", "--"]);
    }
}
