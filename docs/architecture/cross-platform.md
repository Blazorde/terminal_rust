# Cross-platform compatibility

Terminal Rust targets Windows, macOS and Linux by design. Compiling everywhere
is not enough: paths, terminals and available tools differ.

## Shared rules

- Build paths with `Path` and `PathBuf`.
- Prefer `env::current_dir`, `env::var_os` and `std::fs` to system commands.
- Support `HOME` on Unix and `USERPROFILE` on Windows.
- Prefer exit codes and stable formats over localized output.
- Hide expected failures from optional tools while preserving actionable errors
  for explicit user operations.

## Terminals

Simple ANSI sequences work in modern terminals on all three platforms. Advanced
features must still provide:

- a no-color mode;
- an ASCII alternative to icons and Nerd Fonts;
- display-width calculation that does not equate bytes with columns;
- a text fallback when terminal capabilities are unknown.

## External processes

Git is optional. External calls must pass separate arguments to `Command`, avoid
shell command strings, set a working directory or use `git -C`, bound repeated
work through caching or timeouts, and treat a missing executable as an
unavailable capability for prompt enrichment.

## Validation matrix

| Risk | Required evidence |
| --- | --- |
| Path separators | paths constructed with `PathBuf` |
| Spaces in paths | temporary directory containing a space |
| Missing Git | nonexistent executable and no panic |
| Outside a repository | quiet negative result |
| CRLF and LF | line-ending-independent parsing |
| Missing ANSI | plain-text rendering |

CI should retain Windows, Ubuntu and macOS jobs. A job that does not execute any
step is not platform evidence.
