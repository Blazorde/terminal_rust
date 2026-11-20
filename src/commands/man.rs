use std::io;
use std::process::Command;

use crate::render;
use crate::utils::shell_env;

/// Built-in command data a manual page is made from.
pub struct Entry<'a> {
    pub name: &'a str,
    pub usage: &'a str,
    pub summary: &'a str,
}

pub fn page(entry: &Entry) -> String {
    format!(
        "NAME\n    {name} - {summary}\n\nSYNOPSIS\n    {usage}\n\nSEE ALSO\n    help, man\n",
        name = entry.name,
        summary = entry.summary,
        usage = entry.usage,
    )
}

pub fn show(entry: &Entry) -> io::Result<()> {
    render::stdout(page(entry))
}

/// Defer to the operating system's `man` for anything that is not a built-in.
pub fn show_system(args: &[String]) -> io::Result<()> {
    let mut command = Command::new("man");
    shell_env::prepare_child(&mut command);
    let status = match command.args(args).status() {
        Ok(status) => status,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(io::Error::other(format!(
                "no manual entry for {} (the system 'man' program is not installed)",
                args.join(" ")
            )));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(())
    } else {
        // The system `man` already printed why; only report that it failed.
        Err(io::Error::other(format!("man exited with {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_lists_name_synopsis_and_related_commands() {
        let entry = Entry {
            name: "printenv",
            usage: "printenv [variable]",
            summary: "display environment variables",
        };

        assert_eq!(
            page(&entry),
            "NAME\n    printenv - display environment variables\n\n\
             SYNOPSIS\n    printenv [variable]\n\n\
             SEE ALSO\n    help, man\n"
        );
    }
}
