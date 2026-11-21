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
takes precedence over a program of the same name. See
[Pipes and redirections](#pipes-and-redirections) and [Variables](#variables).

## Variables

`$NAME`, `${NAME}` and `$?` (`0` after a success, the exit code of a program
that failed, `127` for an unknown program, `1` for a failure inside the shell) are
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
never split into several arguments. Put `NAME=value` words before a command to
set variables for that command only (`LANG=C sort names.txt`, `PORT=8080 node
app.js`): the program sees them, the shell does not keep them, and the previous
value comes back afterwards.

## Leaving the shell

`exit` closes the shell with the status of the last command. `exit 3` closes it
with code 3 (the code is taken modulo 256, so `exit -1` is 255), which scripts
and callers can test. `exit soon` is an error and the shell stays open. When the
input ends (`Ctrl+D`, the end of a script piped in) the shell also leaves with
the status of the last command.

## Chaining commands

Put several commands on one line with `;`, `&&` and `||`:

```text
cargo build && cargo test
cd project ; ls
make || echo "build failed"
```

`;` always runs the next command, `&&` runs it only if the previous one
succeeded, and `||` only if it failed. They are evaluated from left to right and
`exit` stops the line. Quote an operator to use it as text. A trailing `&` runs
a command in the background, see [Background jobs](#background-jobs).

## Brace expansion

`{a,b,c}` in a word gives one word per alternative, and `{1..5}` or `{a..e}` a
sequence:

```text
mkdir -p project/{src,tests,docs}
cp config.toml{,.bak}
touch note{1..3}.txt
echo {a,b}{1,2}
```

`{01..10}` pads with zeros and `{1..10..3}` steps by three. Quote the braces to
keep them as text (`'{a,b}'`); `${NAME}` is a variable, not a group. A word that
also holds a redirection (`echo {a,b}>file`) is not expanded.

## Command substitution

`$(command)` is replaced by what the command prints, without its final newlines:

```text
echo today is $(date)
cd $(dirname /usr/bin/env)
VERSION=$(git describe --tags)
ls $(find . -name '*.rs')
```

The command can be any line, with pipes and `;`, and substitutions nest. Unquoted,
the result is split into words at spaces and newlines and is never matched
against file names; in double quotes it stays one word (`"$(pwd)"`). In an
assignment (`NAME=$(cmd)`) it is not split. What the command writes on standard
error still goes to the terminal. A result can never add an operator such as `;`
or `>` to the line. Single quotes and `\$(` keep it as text. `cd` inside a
substitution changes the shell's directory (there is no subshell). Backquotes
and `$((...))` arithmetic are not available.

## File name patterns

`*`, `?` and `[...]` in an argument are replaced by the sorted names they match:

```text
ls *.rs
rm build/*.tmp
cat logs/2026-*.txt | wc -l
echo src/*/mod.rs
```

`*` matches any run of characters, `?` one character and `[a-c]` or `[!a-c]` one
character of a set. A pattern matches within one directory level (`*` never
matches `/`), and names that start with `.` need a pattern that starts with `.`.
A pattern that matches nothing stays as written. Quote it (`'*.rs'`) or put a
backslash before it (`\*`, on Linux and macOS) to use it as text. The value of a
variable is never a pattern. There is no `**` and no `{a,b}` yet.

## Pipes and redirections

`|` sends the output of one command to the next, and the commands run together:

```text
ls | grep rs
cargo test 2>&1 | less
printf 'b\na\n' | sort | head -n 1
```

Redirections send a command's input or output to a file:

| Syntax | Meaning |
| --- | --- |
| `< file` | read standard input from `file` |
| `> file` | write standard output to `file` (replaces it) |
| `>> file` | append standard output to `file` |
| `2> file`, `2>> file` | the same for standard error |
| `2>&1` | send standard error where standard output goes |
| `>&2` | send standard output where standard error goes |

```text
sort < names.txt > sorted.txt
cargo build 2> errors.txt
echo done >> log.txt
```

The file name is expanded like any argument (`> "my file.txt"`, `> $HOME/out`).
The status of a pipeline is the status of its last command. Built-in commands
work in a pipeline too: `ls | grep txt`, `echo hi > file`, `echo hi | cat`.
`cat` without a file copies its input; other built-in commands ignore theirs.
Unlike other shells, a built-in such as `cd` or `export` inside a pipeline takes
effect in the shell itself. Here-documents (`<<`) and file descriptors above 2
are not available.

## Background jobs

End a command with `&` to run it in the background and keep the prompt:

```text
npm run dev &
[1] 4242
build & test
```

The shell prints the job number and the process id. A background program reads no
input (its standard input is empty) and its output still goes to the terminal.

| Command | Effect |
| --- | --- |
| `jobs` | list the jobs: `[1]+  Running  npm run dev` |
| `fg [%n]` | wait for a job (the latest by default); `$?` is its exit code |
| `wait [%n]` | wait for one job, or for all of them |
| `kill %n` | stop a job (`kill 1234` still reaches the system `kill`) |

When a job ends the shell says so once, before the next prompt: `[1]+  Done
npm run dev` or `[1]+  Exit 3  ...`. A job is one command or one pipeline: `a &&
b &` is rejected. Programs keep running if you leave the shell. There is no
`Ctrl+Z` and no `bg`: an interactive program has to run in the foreground.

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

Desktop sessions are isolated per tab. Use `Ctrl+Shift+T` (`Command+T` on macOS)
to create a tab, `Ctrl+Shift+W` (`Command+W`) to close it, `Ctrl+Tab` to switch
tabs and `Ctrl+,` (`Command+,`) to open settings. Plain `Ctrl+T` and `Ctrl+W`
are left to the shell, where they swap two characters and delete a word.
Select text with the mouse and press `Ctrl+Shift+C` (or `Ctrl+Insert`, or
`Command+C` on macOS) to copy it; `Ctrl+C` is always the interrupt key. Paste
with the usual shortcut of your system.

More detail: [dashboard](https://github.com/Blazorde/terminal_rust/blob/main/docs/dashboard.md)
and [completion](https://github.com/Blazorde/terminal_rust/blob/main/docs/completion.md).
