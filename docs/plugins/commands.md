# Plugin commands

A plugin command is declared in the manifest and exposed only when its plugin is
enabled, compatible, uniquely named, and authorized. Native commands always win
name conflicts. User commands also remain reserved; two plugins declaring the
same name disable that contribution instead of relying on file-system order.

## Declarative prototype

T117 implements the first safe executable slice: a manifest may attach one
static response to a command.

```toml
permissions = ["command.register", "output.write"]

[[commands]]
name = "hello"
summary = "Print the official plugin greeting"
response = "Hello from the official Terminal Rust plugin!\n"
```

This response is data, not code. It is limited to 4 KiB, cannot contain NUL or
escape characters, accepts no arguments, launches no process, reads no file,
and performs no network operation. The host writes it through the normal render
capture, so CLI and `ShellEngine` callers receive the same typed output.

The command becomes available only when configuration contains both activation
and consent:

```toml
[plugins]
enabled = ["example.hello"]

[plugins.grants]
"example.hello" = ["command.register", "output.write"]
```

Removing the ID from `enabled` hides the command immediately while preserving
its grant record and plugin settings. Native shell commands continue to work.

## Dispatcher integration

The command lookup order is:

1. native command table;
2. structured user commands;
3. authorized declarative plugin commands;
4. typed `unknown command` failure.

Completion reads the same active command names from `ShellEngine`. A plugin
entry that is disabled, incompatible, invalid, ungranted, duplicated, or in
conflict is never returned as executable. Invoking a uniquely named but
ungranted declarative command returns the explicit permission denial; it is not
misreported as an unknown command.

## Future runtime commands

The Rust SDK template still implements `Plugin::execute` for contract tests, but
the host does not load its `.rlib`. A future isolated runtime will transport a
serialized `CommandRequest` and `CommandResponse`, enforce a deadline, and use
the same permission and compatibility gates. The declarative path remains a
small useful runtime of its own; it must not grow into a script language.

Arguments will arrive as an already parsed vector, never a raw shell string.
Plugin output remains UTF-8 text plus bounded semantic styles, and ANSI supplied
by a plugin is treated as text. Directory changes use host-issued resource
tokens. A plugin failure never becomes a core panic.
