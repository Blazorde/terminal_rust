# Usage

Run `help` inside Terminal Rust for the authoritative command list.

| Command | Purpose |
| --- | --- |
| `pwd` | Print the working directory |
| `cd`, `ls`, `mkdir`, `touch`, `cat`, `rm` | Portable file operations |
| `clear` | Clear an ANSI-capable terminal |
| `dashboard` | Open the live system and project dashboard |
| `git status\|branch\|log` | Run an allowlisted read-only Git operation |
| `theme list`, `theme preview` | Inspect built-in prompt themes |
| `plugins list`, `plugin list` | Inspect plugin manifests and permissions |
| `plugins help`, `plugin help` | Explain what plugins support and how to inspect them |
| `commands list` | Inspect configured custom commands |
| `commands help` | Explain custom commands and where to configure them |
| `echo [-n] [text...]` | Print text, without a trailing newline with `-n` |
| `whoami` | Prints the username of the current user. |
| `arch` | Prints the system architecture. |
| `nproc` | Prints the number of processing units available. |
| `uptime` | Prints how long the system has been running. |
| `date [format]` | Prints the current date and time. |
| `hostname` | Prints the name of the current host.  |
| `nano`, `nvim`, `neovim` | Open a terminal editor; `neovim` aliases `nvim`. |
| `printenv [var]` | Prints the environment variables.  |
| `man <command>` | Shows the manual of a built-in command, or the system manual page for any other topic |
| `exit`, `quit` | Close the session |

Install [Blazorde's Neovim configuration](Neovim) to use the editor with the
published CodeOpsHQ setup.

## Running other programs

A command that is not built in runs the program of that name found in your
`PATH`, so tools such as `cargo`, `python` or `htop` work directly:

```text
cargo build --release
python3 script.py
```

The program shares the terminal, the environment and the current directory.
`Ctrl+C` stops the program and returns to the prompt. A built-in command always
takes precedence over a program of the same name. Pipes, redirections and
variables are not available yet.

## Variables

`$NAME`, `${NAME}` and `$?` (`0` after a success, `1` after a failure) are
expanded in command lines, and a leading `~` stands for your home directory.
Double quotes expand variables, single quotes do not, and `\$` writes a literal
dollar sign:

```text
NAME=world
echo "hello $NAME"
cd ~/code
export EDITOR=nvim
PATH=$PATH:$HOME/bin
unset NAME
```

`NAME=value` creates a variable that only the shell sees. `export NAME` (or
`export NAME=value`) also passes it to the programs you start, and `unset NAME`
removes it. A variable that is not set expands to nothing, and an expansion is
never split into several arguments. Setting a variable for a single command
(`NAME=value command`) and `*` patterns are not available yet.

## Chaining commands

Put several commands on one line with `;`, `&&` and `||`:

```text
cargo build && cargo test
cd project ; ls
make || echo "build failed"
```

`;` always runs the next command, `&&` runs it only if the previous one
succeeded, and `||` only if it failed. They are evaluated from left to right and
`exit` stops the line. Quote an operator to use it as text. Pipes (`|`),
redirections (`<`, `>`) and background jobs (`&`) are not available yet: the
shell says so instead of passing them to a program.

## Pasting several lines

A block of several lines runs like the same lines typed one after another, in
order. The same is true of a command you type while another program is still
running: it runs when the prompt comes back.

```text
echo first
make
echo done
```

Each line runs on its own, so a line that fails is reported and the next lines
still run. To stop at the first failure, join the commands with `&&` on one
line (`make && echo done`), or put them in a script, which stops at the first
failing command.

## Paths containing spaces

Quote a path containing spaces with single or double quotes so the command
receives it as one argument:

```text
cat "notes from today.txt"
rm 'notes from today.txt'
rm -d "old project"
```

The quotes are removed by the command parser. You can also escape spaces with
a backslash (`cat notes\ from\ today.txt`). Native path separators are
preserved, including backslashes in Windows paths.

The dashboard refreshes once per second. Press `q`, `Escape` or `Ctrl+C` to
leave it. Redirected output and limited terminals receive a stable text
snapshot.

`Tab` and `Shift+Tab` cycle completions. Accept a dim history suggestion with
`Right Arrow` and hide it with `Left Arrow`.

Plugins are declarative extensions described by local manifests. `plugins list`
shows their status and requested permissions; `plugin` is an alias for
`plugins`. Custom commands are shortcuts from the user configuration:
`commands list` shows configured shortcuts, and `commands help` explains them.
See [custom commands](../docs/custom-commands.md) and the
[plugin tutorial](../docs/plugins/getting-started.md) for setup details.

Desktop sessions are isolated per tab. Use `Ctrl/Command+T` to create a tab,
`Ctrl/Command+W` to close it, `Ctrl+Tab` to switch tabs and `Ctrl/Command+,` to
open settings.

More detail: [dashboard](https://github.com/Blazorde/terminal_rust/blob/main/docs/dashboard.md)
and [completion](https://github.com/Blazorde/terminal_rust/blob/main/docs/completion.md).
