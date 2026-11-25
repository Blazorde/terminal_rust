# Changelog

All notable changes to Terminal Rust are documented in this file. Versions use
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- The shell can be started like other shells (ADR-0034): `-c 'string' [name args...]`, `terminal_rust file args...`
  (also through `#!/usr/bin/env terminal_rust`), `-i`, `-l` / `--login` and `--`.
- An interactive shell reads `~/.terminal_rust_rc`, the desktop app included.
- On Unix, a login shell takes the environment of `/etc/profile` and `~/.profile`, run by `/bin/sh`.
- Operators on the value of a variable (ADR-0030): `${f%.txt}`, `${path##*/}`, `${s/old/new}`, `${s//old/new}`,
  `${s:1:3}`, `${s: -3}`, `${s^^}`, `${s,,}`, and slices of arrays and parameters (`${arr[@]:1}`, `${@:2}`).
- `let` and `(( expression ))` with assignments, `+=`, `++`, `--` and the comma; the status is `0` for a non-zero
  value and `1` otherwise (ADR-0033).

### Changed

- Piped input runs without a prompt and without the startup script; `-i` keeps both.
- A first argument that is an existing file now runs as a script. A built-in or a program of an absolute
  entry of `PATH`, written without a path, keeps priority over a file of the same name, and that program is
  the one that runs even when a relative entry comes first. A path to a program
  (`terminal_rust /usr/bin/ls`) is read as a script and gives status `126`.

### Fixed

- Windows: a pattern after a path written with backslashes (`~/*.txt`, `$PWD/*.rs`) now expands, and a
  backslash in a `case` or `[[ ]]` pattern is a path separator, as ADR-0012 says for file name patterns.

### Changed

- CI caches compiled Cargo dependencies, cancels the superseded runs of a pull request (never those of
  `main` or `develop`), runs formatting and Clippy before the long steps, reports every failing test in one
  run, and skips the platform jobs when only documentation changed. The branch rule tests now run in CI.

## [1.12.0] - 2026-11-22

### Added

- Indexed arrays (ADR-0029): `files=(*.rs)`, `files+=(x)`, `files[2]=x`, `${files[1]}`, `${files[@]}`,
  `${#files[@]}`, `${!files[@]}`, and `"${files[@]}"` gives one word per element.
- `trap 'command' EXIT` runs a command when the shell ends (`trap - EXIT` removes it, `trap`
  shows it) and `a ** b` is the power in `$(( ))` (ADR-0028). Signal traps are refused with a message.
- Subshells `( commands )` (ADR-0027): the variables, functions, directory and exports
  they change are put back when they end, and `exit` leaves only them.
- `[[ expression ]]` conditions (ADR-0026): patterns with `==` and `!=`, regular expressions with
  `=~`, `<` and `>` on strings, `&&`, `||`, `!` and parentheses, no word splitting.

### Added

- `! command` turns the status of a command around (`if ! test -f x; then …`), `command -v`,
  `command -V` and `command name args` (skips a function of the same name), `exec command`
  (runs it and leaves the shell with its status), `$PPID` and `$RANDOM` (ADR-0025).

## [1.11.0] - 2026-11-22

### Added

- `$$` (the process id of the shell), `$!` (the process id of the last job started with
  `&`, empty before any) and backquote command substitution `` `command` `` as the old
  spelling of `$(command)`. A line that ends with the closing backquote of a pair is no
  longer taken as a continuation line.

## [1.10.0] - 2026-11-22

### Added

- Job control (ADR-0024): `Ctrl+Z` stops the program that runs in the foreground and
  gives the prompt back (`[1]+  Stopped  vim notes.txt`), `fg` brings it back, `bg`
  continues it in the background, `jobs` shows `Stopped`, and `exit` warns once about
  stopped jobs. `Ctrl+C` now reaches the program and not the shell. It is written in
  safe Rust (`rustix`, `nix`, `signal-hook`), and only active when the shell is on a
  terminal.

- `${=NAME}` splits the value of a variable into one word per blank-separated piece
  (ADR-0009): `FLAGS="-a -l"; ls ${=FLAGS}`, `for f in ${=LIST}; do …; done`. Plain
  `$NAME` stays a single word, so a value with spaces is still safe.

### Changed

- The release workflow retries the downloads of the bundled runtime (Nano sources and
  binary, Git licenses) up to 6 times on Linux and macOS and 8 times on Windows, 20
  seconds apart. A host that was unreachable for a minute made the last releases fail and
  need a manual re-run.

## [1.9.1] - 2026-11-22

### Fixed

- `ls` accepts `--color` and `--color=WHEN` (it prints no colors), so the usual
  `alias ls='ls --color=auto'` works instead of failing with `unknown option`.
- `alias` no longer prints `Alias 'name' saved for this session.` when it runs inside a
  file (a startup script, a `source`d file): the confirmation is for the prompt, and a
  startup script with aliases said it at every start.

## [1.9.0] - 2026-11-22

### Added

- Here-documents `<<WORD` and `<<-WORD` (quoted words keep the body literal) and
  here-strings `<<<` (ADR-0023): the prompt asks for more until the delimiter, in the
  terminal, in piped input, in scripts and in `source`d files, and the body expands
  variables and `$(…)` like double-quoted text.
- Redirections and pipes on whole blocks (ADR-0022): `while read line; do …; done <
  file`, `cmd | while read x; do …; done`, `for … done > file`, `{ a; b; } | sort`,
  `if … fi | cmd`, `case … esac >> log`. `read` takes one line at a time from the
  block's input, `cat` takes what is left, programs started inside the block are
  collected, and the end of the input is the end of the file, not the keyboard.

### Fixed

- A block that follows `&&` or `||` ran its first command with the connector of the
  block itself: `cmd || { echo failed; }` printed nothing, `false || if false; then …`
  took its branch, and `false || while …` never ended. The first command of a block
  (`{`, `if`, `while`, `until`, `case`, a function body) now runs unconditionally once the
  block is entered.

## [1.8.0] - 2026-11-21

### Added

- Functions and positional parameters (ADR-0021): `name() { …; }`, `function name { … }`
  and `{ …; }` groups; `$1`…`$9`, `${1:-x}`, `$#`, `$*`, `$@` and `"$@"` (one word per
  argument); `shift`, `return [N]` and `local`; recursion limited to 64 calls;
  `unset -f`, `type` and completion know functions; `source file args` passes `$1`….
  `$(( … ))` now runs substitutions inside the expression first, and substitutions may
  nest 24 deep (they were 8).
- `case WORD in PATTERN|PATTERN) …;; … esac` (ADR-0016 amended): patterns with `*`
  `?` `[...]`, several patterns per arm, arms on one line or several, nesting, and the
  `>> ` prompt until `esac`. A pattern also matches `/` and a leading `.`; quoted or
  variable patterns are literal.
- Recursive patterns (ADR-0020): a path segment `**` goes through any number of
  directories, so `ls **/*.rs` and `for f in **/*.log` work, and a trailing `**` lists
  everything below. Hidden directories are skipped, links are not followed, and the
  walk is bounded (32 levels, 50 000 paths). A lone `**` used to act like `*`.
- `alias name=value`, the form most people type (`alias ll="ls -la"`), next to
  `alias name command...`; one `alias` can define several names, and the value is
  read like a command line.
- `echo` reads escapes with `-e` (`\n`, `\t`, `\e`, `\xHH`, `\0NNN`, `\uHHHH`,
  `\c`), accepts grouped options (`-ne`) and `-E`, and `pwd` accepts `-L` and `-P`.
- Git completion (ADR-0019): `Tab` after `git ` completes the subcommands, and after
  `checkout`, `switch`, `merge`, `rebase`, `branch`, `diff`, `log`, `push`, `pull`
  and a few more it completes branches, remote branches and tags, read from the
  repository files without starting Git.

### Fixed

- `PWD` and `OLDPWD` follow `cd`. They kept the value of the parent process, so
  `$PWD` and the programs the shell started saw a stale directory after a `cd`. A
  stale `PWD` is replaced at start-up, and `cd -` now prints the directory it goes
  to.
- File name completion no longer offers hidden entries (`.git/`, `.cache`) unless the
  name being typed starts with a dot.

## [1.7.0] - 2026-11-21

### Added

- History: `history [N]` lists the lines typed at the prompt, and `!!`, `!$`, `!^`,
  `!N`, `!-N` and `!prefix` stand for earlier lines (ADR-0018). The expanded line
  is shown and recorded; a missing event runs nothing.
- `source file` and `. file` run a file in the current shell (any file name, stops
  at the first error and names the line); `read [-r] [-p prompt] [name...]` reads
  a line from a pipe, a redirection or the terminal into variables.
- Arithmetic expansion `$((…))` on 64-bit integers (ADR-0017): `+ - * / %`,
  parentheses, signs, comparisons and `&&` `||`, variables by name, hexadecimal
  numbers. Overflow and division by zero are errors, never wrapped values.
- Parameter expansion in `${…}`: `${NAME:-default}`, `${NAME-default}`,
  `${NAME:+alternative}`, `${NAME:?message}` and `${#NAME}`, with nested words,
  quotes and variables. `${NAME:=word}` and the trimming forms are refused with a
  clear message.
- `type` and `which` say what a name runs: keyword, built-in, alias, custom
  command, plugin, or the program found in `PATH` with its path (a missing name is
  reported and gives status 1).
- `test` and `[ … ]` are built in (ADR-0016): `-e -f -d -s -r -w -x -L`, `-z -n`,
  `= == !=`, `-eq -ne -lt -le -gt -ge`, `!`, `-a` and `-o`. Status 0 when true, 1
  when false without a message, 2 for a mistake. Conditions in `if` and `while`
  now work on Windows, where there is no `test` program.

## [1.6.0] - 2026-11-21

### Added

- Control flow ([ADR-0016](docs/decisions/0016-control-flow.md)): `for NAME in
  WORDS; do …; done`, `while` and `until` loops, and `if … then … elif … else …
  fi`, which nest and hold chains and pipelines. New built-in commands `break`,
  `continue`, `true` and `false`. A newline now works like `;`, a block can be
  typed or pasted over several lines (the shell asks for more with `>> ` until
  `done` or `fi`) and used in `.trs` scripts, and `Ctrl+C` stops a loop between
  two commands. A failing condition is not reported as an error. The structure
  is checked before anything runs.
- Brace expansion ([ADR-0015](docs/decisions/0015-brace-expansion.md)):
  `{a,b,c}`, nested groups and several groups per word, and sequences `{1..5}`,
  `{a..e}`, `{01..10}`, `{1..10..3}`. Quotes and `${NAME}` keep braces literal.
- `mkdir` and `touch` accept several targets and keep going after a failure, and
  `mkdir -p` creates the missing parents (and accepts an existing directory).
  They accepted a single argument before, which made them unusable with `*` and
  `{...}`.

### Changed

- `Ctrl+C` during a line now also abandons the commands that were still to run on
  that line, as in other shells.

### Fixed

- `cat` accepts several files (it read only the first one, so `cat *.txt` was
  wrong), prints those it can read and names each one it cannot, and `-` stands
  for the input of the pipeline: `echo x | cat a.txt - b.txt`.

## [1.5.0] - 2026-11-21

### Added

- Command substitution ([ADR-0014](docs/decisions/0014-command-substitution.md)):
  `$(command)` is replaced by the output of the command, which can be a whole
  line with pipes and chains, and substitutions nest. Unquoted results are split
  into words (and never matched against file names), quoted ones stay whole, and
  a result can never inject an operator. Programs and Git write into the
  substitution like built-in commands do. Backquotes and `$((...))` are not
  supported.
- Desktop: `Ctrl+Shift+C` (or `Ctrl+Insert`, `Command+C` on macOS) copies the
  selected text to the clipboard. Without a selection the keys keep their
  terminal meaning, and `Ctrl+C` is still the interrupt key.

### Security

- The error messages of the shell quote what you typed and the names it could
  not find (`cd`, `cat`, an unknown command...). A name holding a terminal escape
  sequence now has its control characters replaced by `?` there too, in the text
  written on standard error and in the error returned by the engine. Names shown
  by `ls`, `pwd` and the prompt were already protected.

### Fixed

- `git` in a pipeline or with a redirection: its output went straight to the
  terminal, so `git log | head` or `git diff > patch.txt` did not work. Git now
  runs as a program whose streams are connected like any other.
- Desktop: two copies of the application starting together could delete and
  extract the bundled Git runtime at the same time and leave a broken one. The
  installation now takes a lock file shared by all processes (released when the
  process ends, even after a crash), and re-checks the runtime once it holds it.
- Desktop: the bundled runtime archive, several hundred megabytes, was read in
  full at every start to compute its digest. The digest is now kept with the size
  and date of the archive and only recomputed when they change.

## [1.4.0] - 2026-11-21

### Added

- Background jobs ([ADR-0013](docs/decisions/0013-background-jobs.md)): a trailing
  `&` runs a command or a pipeline in the background and prints `[1] pid`. New
  commands `jobs`, `fg [%n]`, `wait [%n]` and `kill %n`; a finished job is
  reported once before the next prompt (`[1]+  Done  command`, `Exit 3`). A
  background program reads no input. There is no `Ctrl+Z` or `bg`, and
  `a && b &` is rejected.
- `NAME=value command`: variables set for a single command (and for one stage of
  a pipeline), restored afterwards.
- `exit [code]`: the shell leaves with the given code (modulo 256), or with the
  status of the last command; an invalid code is an error and the shell stays
  open. The shell also leaves with the status of the last command when its input
  ends, and `terminal_rust <command>` returns the exact code of the command.
- Completion offers the programs found in `PATH` next to the built-in commands,
  also after `;`, `|`, `&&` and `||` (it used to work only at the start of the
  line), and completes environment variable names after a `$` or `${`.

### Changed

- `$?` is now the exact exit code of the last program (`3` after `exit 3`, `137`
  after a kill), `127` for a program that does not exist, and the code of the
  last program of a pipeline. It used to be only `0` or `1`. A command that fails
  inside the shell is still `1`, and so is a line that cannot be parsed.
- A script that ends on a failing command now exits with that command's status
  instead of 0.

## [1.3.0] - 2026-11-21

### Added

- File name patterns ([ADR-0012](docs/decisions/0012-filename-patterns.md)): `*`,
  `?`, `[abc]`, `[a-z]` and `[!a-z]` in an argument are replaced by the sorted
  matching names, one directory level at a time. Hidden names need a leading `.`,
  a pattern without a match stays as written, and quotes, a backslash (outside
  Windows) and variable values keep a wildcard literal.
- Pipes and redirections ([ADR-0011](docs/decisions/0011-pipelines-and-redirections.md)):
  `a | b | c`, `< file`, `> file`, `>> file`, `2> file`, `2>> file`, `2>&1` and
  `>&2`. Programs are connected with real pipes and run together, built-in
  commands can produce the input of a program or read a pipe (`cat` without a
  file), the status of a pipeline is the one of its last command, and a syntax
  error rejects the line before anything runs. Background jobs (`&`) are still
  rejected.

### Changed

- Builds are reproducible: the crates declare `rust-version = "1.95"`, the lowest
  version that compiles (the `sysinfo` dependency requires it), the CI uses the
  fixed toolchain 1.95.0 instead of `stable`, cargo runs with `--locked`, and
  every GitHub Action is pinned to a commit SHA.

### Fixed

- Plugin discovery is tolerant: a plugin folder that cannot be read is reported
  as an error and no longer disables every plugin, user command and alias. When
  several compatible versions of a plugin are installed only the newest is used
  (the others are `superseded`) instead of blocking all its commands as a name
  conflict, and a manifest whose `id` differs from its folder name is rejected.
- Desktop: `Ctrl+W` (delete a word) and `Ctrl+T` (swap two characters) no longer
  close or open a tab, they reach the shell. On Linux and Windows the tab
  shortcuts are now `Ctrl+Shift+T` and `Ctrl+Shift+W`, as in other terminals;
  macOS keeps `Command+T` and `Command+W`.
- Desktop: the first output of a new session (the prompt) could be lost when it
  arrived before the tab knew its session id. It is now held and shown in order.
- Desktop: starting a session and writing to it no longer run on the thread that
  draws the window, so the interface stays responsive during the first start,
  which can extract the Git runtime, or when the shell is slow to read input. A
  blocked write no longer holds up the other sessions.
- The shell no longer exits when its current directory is deleted from another
  terminal: the prompt shows `(deleted)` and `cd` can leave the directory.
- `cat` shows a file that is not valid UTF-8 (a binary file, Latin-1 text) with
  U+FFFD in place of the bad bytes, instead of failing.
- `touch` now updates the modification time of an existing file.
- The prompt no longer freezes in a very large repository or on a slow disk:
  `git status` is stopped after 1.5 seconds and the Git segment is left out for
  that prompt. Git is not queried at all when no configured prompt segment shows
  it (`prompt.segments` and `prompt.right_segments`).

## [1.2.2] - 2026-11-21

### Security

- A file or directory name holding a terminal escape sequence could rewrite the
  window title or the screen when listed with `ls`, shown by `pwd`, in the prompt
  path, in the dashboard or in `plugins list`. Control characters (C0, C1, DEL,
  bidirectional overrides) in these texts, and in the response of a plugin
  command, are now replaced by `?` before they are displayed.

### Fixed

- Saving the shell state (permanent aliases, startup script) or the desktop
  settings is now atomic. The old code moved the existing file to a `.bak` before
  moving the new one, so a crash in between left no file at all, and two shells
  saving at once shared the same temporary name and could corrupt each other. The
  data is now written to a unique temporary file, flushed, and renamed over the
  target in one step, keeping the file's permissions. No `.bak` file is left.
- The desktop tab froze for good, and its memory grew without limit, when a
  program printed an `ESC ] 777 ;` sequence that was not a prompt layout, such as
  the standard notification `printf '\e]777;notify;Build;done\a'`. The prompt
  filter now ends at the sequence terminator (BEL or `ESC \`), passes anything
  else through to the terminal, and releases a sequence longer than 4 KiB.
  The filter is a separate module with its own tests (`npm test`).
- Custom commands and aliases of the `process` kind ran with their output
  captured and no standard input, so interactive programs (a prompt, `python`,
  `ssh`, `less`) could not work and the output only appeared at the end. They now
  share the terminal like any other program, and a failure reports the exact exit
  status (`name: exited with status 3`) instead of a doubled message.
- Pressing Tab after a no-break space or another multi-byte whitespace crashed
  the shell, because the completion assumed a whitespace takes one byte. The
  word start is now computed from the character width and the cursor position is
  moved back to a character boundary.
- The remove command could delete what the user did not mean to: asking it to
  remove the current directory or its parent emptied them. Those two targets and
  the root directory are now refused, before anything is removed, as in GNU `rm`.
- `rm` now keeps going after a failure and reports every failing target instead of
  stopping at the first one, can remove a broken symlink, and never follows a
  symlink to a directory.
- A `rm` confirmation read from a piped standard input blocked the shell for ever,
  and the question was not shown in a terminal until after the answer.

## [1.2.1] - 2026-11-20

### Added

- `scripts/ci-local.sh` replays the Linux gate of the CI, with the Security checks,
  in the pinned Docker image and summarizes each step, for use when hosted
  runners are unavailable.

### Fixed

- A command typed while another program was still running was never executed:
  the terminal turned Enter into a line feed, and the shell bound that key
  (`Ctrl+J`) to "insert a new line" instead of accepting the line. `Ctrl+J` now
  validates the line like Enter, as in other shells. Multi-line input still uses
  a trailing backtick, and `Shift+Enter` in the desktop terminal.
- Lines pasted or typed faster than the shell reads them were lost: only the
  first command of a paste ran. The line editor read ahead into a buffer that it
  dropped after each prompt; enabling its `buffer-redux` feature keeps that
  buffer between prompts (Unix).

## [1.2.0] - 2026-11-20

### Added

- Command chaining: `;` runs the next command, `&&` only after a success and
  `||` only after a failure. Each command is expanded right before it runs, so
  `$?` and variables set earlier on the line are visible, and output stays in the
  order of the commands even when programs are involved. See ADR-0010.

### Changed

- An unquoted `|`, `<`, `>` or `&` is now rejected with a clear message instead
  of being passed to a program as an argument, until pipes, redirections and
  background jobs are designed.

- Variables in command lines: `$NAME`, `${NAME}`, `$?` and a leading `~` are
  expanded outside and inside double quotes (not inside single quotes), `\$`
  writes a dollar sign, and `NAME=value` sets a variable. `export NAME[=value]`
  passes a variable to the programs the shell starts, `unset NAME` removes it,
  and `PATH=$PATH:dir` updates the search path. See ADR-0009.
- Any command that is not built in now runs the program of that name found in
  `PATH` (`PATHEXT` is honored on Windows), with the terminal streams, the
  environment and the current directory of the shell. A missing program still
  reports `unknown command`, and a failing one reports its exit status. See
  ADR-0008.
- `Ctrl+C` no longer ends the interactive shell while a program is running.

## [1.1.0] - 2026-11-20

### Added

- Bundled Git, Nano and Neovim into desktop releases so full Git workflows and
  editor commands work without separate installations.
- Added `nano`, `nvim` and `neovim` commands and made the bundled editors
  available to Git's configured interactive editor.
- Added an `echo` command that prints its arguments separated by spaces.
- Added a `man` command: built-in commands show their name and synopsis, and any
  other topic is handed to the system `man` when it is installed.
- Added a whoami command that prints the current user’s username.
- Added an arch command that prints the system architecture.
- Added an nproc command that prints the number of available processing units.
- Added an uptime command that prints how long the system has been running.
- Added a date command that prints the current date and time.
- Added a hostname command that prints the current host’s name.
- Added a printenv command that prints the environment variables.
- Added Linux terminal integration to the desktop app: `--working-directory` /
  `-w`, `-e` and `--` launch options for the first session, a `TerminalEmulator`
  desktop entry, and an `x-terminal-emulator` alternative registered with a low
  priority by the DEB. See the Linux wiki page.

### Changed

- Replaced the README cover with a lighter hand-drawn illustration.
- Renamed the `refractor/` branch prefix to `refactor/` in the branch and pull
  request policies.

### Fixed

- Fixed the desktop crate failing to compile on Linux and macOS.
- Restored the CI: bundled runtime preparation (Nano version parsing, Git
  license file name, Windows download retries), resource permissions and the
  desktop PTY tests on Linux and macOS.
- Replaced the yanked `clipboard-win` 5.4.1 with 5.4.3 and refreshed the
  third-party notices.

## [1.0.0-alpha.1] - 2026-10-13

### Added

- Documented the post-MVP plugin vision, extension boundaries, permission model
  and staged roadmap inspired by Neovim without enabling third-party code.
- Defined the target plugin architecture, declarative-first lifecycle, stable
  message boundary, future source layout and cross-platform compatibility rules.
- Specified the proposed plugin API DTOs, host capabilities, error model,
  version negotiation and runtime-independent validation rules.
- Added a validated, backward-compatible plugin configuration schema with
  explicit activation and preserved plugin-owned settings.
- Added a cross-platform local plugin registry, validated versioned manifests,
  and a read-only `plugins list` command with explicit health states.
- Centralized native command syntax and summaries so dispatch, generated help,
  and completion can share the same future plugin-aware registry boundary.
- Added validated prompt-segment declarations with bounded execution and cache
  budgets while keeping all rendering under host control.
- Added a runtime-independent plugin SDK and a copyable, compiling `hello`
  plugin template with a validated manifest and tests.
- Added structured custom commands for built-in aliases and directly spawned
  processes, including listing, completion, conflict checks, and safe arguments.
- Compared Lua, Rhai, and WebAssembly for future scripting and proposed a
  bounded Rhai experiment after the plugin permission model is complete.
- Added an optional Ratatui layout prototype with two focusable panes,
  horizontal and vertical splits, keyboard shortcuts, and resize handling.
- Prioritized future project detection and task shortcuts while preserving
  native developer tools and the existing no-implicit-shell boundary.
- Added a fail-closed plugin permission gate, sensitive capability requests,
  visible permission review, and explicit denial diagnostics.
- Added explicit Terminal Rust version requirements to plugin manifests and
  actionable API/application compatibility diagnostics in the registry.
- Defined a local-first plugin catalog, transactional installation lifecycle,
  command surface, and security boundary for future discovery work.
- Added an English beginner plugin tutorial covering the template, manifest,
  typed command, tests, local discovery, debugging, and responsible sharing.
- Added the first official declarative plugin command runtime with explicit
  grants, static bounded responses, command dispatch, and disable-safe tests.
- Added a reusable `ShellEngine` library API with typed execution status,
  captured standard streams, errors, and directory-change metadata for CLI and
  future desktop consumers.
- Added the first Tauri desktop window backed by a native Rust-owned PTY, with
  real shell input and output, keyboard focus, resize handling, session restart,
  an xterm.js renderer, and an Atelier-inspired responsive interface.
- Added desktop build and native PTY integration checks to the cross-platform
  CI matrix.
- Added a keyboard-accessible desktop settings drawer with six local themes,
  terminal typography and cursor controls, compact density, optional status
  bar, and versioned persistence with validated safe fallbacks.
- Added Rust-owned prompt context for user, host, path, Git, last status,
  project runtime, command duration, and local time, with configurable left and
  right layouts shared with the desktop status bar.
- Added a live cross-platform system dashboard that refreshes in place until
  `q`, `Escape` or `Ctrl+C`, with static fallbacks for redirected, dumb and
  undersized terminals.
- Added dim inline history suggestions accepted with `Right Arrow`, circular
  `Tab`/`Shift+Tab` completion and tested quoted Unicode path handling.
- Added keyboard-accessible desktop tabs backed by independent native PTY
  sessions, with isolated input, output, working directory, history and prompt
  context.

### Changed

- Redesigned the desktop shell around a frameless native window with branded
  title chrome, clear PTY state, compact session tabs and responsive controls.
- Desktop prompts now keep the editable line compact while the status bar owns
  full path, user, host, runtime and timing context.
- Interactive `ls` output now wraps to the available width and uses portable
  semantic colors for directories, project manifests, source, tools and archives.
- Linux desktop packaging now declares the complete native icon set and embeds
  the MIT license alongside third-party notices and bundled configuration.
- Expanded the desktop palette to Graphite, Bone, Nord, Amber, Moss and Dusk,
  while migrating the previous Paper and Midnight settings automatically.
- The desktop terminal now refreshes xterm font metrics and dimensions after
  font, density, tab, viewport and window changes so typed text stays aligned
  with every cursor style.

### Fixed

- Prevented structured prompt metadata from appearing as raw JSON or shifting
  Rustyline's visible prompt width in Windows and Linux PTY sessions.
- Serialized desktop keyboard writes so rapid input and long paths cannot reach
  the native PTY out of order or lose characters.
- Restored native drag and edge/corner resize behavior for the frameless window.
- Standardized contributor, architecture, configuration, desktop, plugin,
  installation, release and troubleshooting documentation in concise English.
- Clarified the source commands that open the native Tauri terminal window and
  added the project's optional Ko-fi support link.
- The CLI now executes built-in commands through the same public Rust engine
  intended for the desktop adapter.
- The desktop terminal surface now fills the native window without an outer
  decorative canvas, redundant border, or shadow.
- Desktop themes now meet WCAG AA text contrast, the settings dialog traps and
  restores keyboard focus, reduced motion has targeted fallbacks, and frontend
  dependencies are emitted as bounded chunks.

## [0.3.0] - 2026-10-10

This release introduces the configurable, segmented prompt engine while
preserving graceful fallbacks on Windows, Linux, and macOS.

### Added

- Defined the versioned prompt customization and theme contracts with default
  TOML examples and icon-free fallbacks.
- Added a reusable prompt segment model that keeps data, semantic styles, and
  optional icons independent from ANSI rendering.
- Added a portable ANSI prompt renderer with foreground, background, emphasis,
  per-segment resets, and automatic plain-text fallback.
- Added an injectable semantic color palette so themes can remap prompt colors
  without changing path or Git segment logic.
- Added centralized Nerd Font icons with deterministic ASCII fallbacks and
  `auto`, `always`, and `never` display modes.
- Added validated, cross-platform TOML user configuration loading with safe
  defaults and actionable file-specific diagnostics.
- Added configurable prompt segment selection and ordering, including separator,
  newline, path style, Git status marker, and symbol options.
- Added selectable `default`, `minimal`, `git-focus`, and `powerline` theme
  presets, including automatic ASCII fallback without a Nerd Font.
- Added `theme list` and `theme preview <name>` commands for safe theme
  discovery without changing the user configuration.
- Verified user-level installers for Windows x86_64, Linux x86_64, and macOS
  Apple Silicon, with authenticated private-release support.

### Improved

- Added the repository's MIT license file and bundled the license, README,
  changelog, and configuration examples in release archives.
- Added end-to-end CLI coverage and a reproducible MVP quality checklist for
  public commands and terminal fallbacks.
- Added copyable onboarding, a supported-platform matrix, and the complete
  contributor validation workflow to the README.
- Added installer syntax validation to the cross-platform CI matrix.
- Removed duplicate CI matrices for work-branch pushes; the full matrix remains
  required on pull requests and protected-branch updates.
- Centralized prompt rendering so the interactive shell and theme previews use
  the same ANSI, icon, separator, and fallback behavior.

### Known limitations

- Custom theme files are not loaded yet; this release provides four built-in
  presets.
- Powerline glyphs and prompt icons require a compatible Nerd Font and explicit
  capability confirmation; ASCII fallbacks remain available everywhere.
- Terminal Rust remains a pedagogical mini-shell and does not replace Bash,
  PowerShell, or Zsh.

## [0.2.0] - 2026-10-09

### Added

- Persistent command history and the `clear` built-in command.
- Non-blocking Git repository detection and read-only `git status`, `git branch`,
  and `git log` shortcuts.
- A cross-platform `dashboard` command for CPU, memory, storage, network, and Git
  context.
- A portable Fastfetch preset and setup documentation.
- Cross-platform CLI integration tests and release builds for Windows, Linux,
  and macOS.

### Improved

- Added project architecture, decision records, contribution guidance, and
  release documentation.
- Added automated GitHub Release publication with checksums for version tags.

### Known limitations

- Terminal Rust remains a pedagogical mini-shell and does not yet replace Bash,
  PowerShell, or Zsh.
- The prompt segment system inspired by Oh My Posh is planned but not included
  in this release.

## [0.1.0] - 2026-06-20

### Added

- First public release with the interactive shell, command history for the
  current session, completion, and quoted-argument parsing.
- Core `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`, and `rm` commands.
- Cross-platform CI for formatting, compilation, Clippy, and tests.
- Windows x86_64 binary and checksum attached to the GitHub Release.
