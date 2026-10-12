# Local plugin catalog

- Status: long-term design, not implemented
- First milestone: local files only
- Explicit non-goal: hosted marketplace service

The first catalog is a user-selected TOML file. Terminal Rust never downloads a
default catalog, contacts a server at startup, or treats a catalog entry as
trusted. Discovery, installation, activation, and permission consent are four
separate actions.

## Catalog format

```toml
schema_version = 1
generated_at = "2026-10-12T00:00:00Z"

[[plugins]]
id = "example.hello"
name = "Hello plugin"
description = "Minimal Terminal Rust command plugin"
repository = "https://github.com/example/terminal-rust-hello"
license = "MIT"

[[plugins.versions]]
version = "0.1.0"
host_api = "^1.0"
terminal_version = ">=0.3, <2.0"
source = "artifacts/example.hello-0.1.0.tar.gz"
sha256 = "<64 lowercase hexadecimal characters>"
size = 12345
permissions = ["command.register", "output.write"]
```

Rules:

- `schema_version`, plugin IDs, SemVer values, permissions, and hashes use the
  same strict validation as installed manifests.
- Plugin IDs and versions are unique. Entries are sorted by ID and descending
  version when displayed; file order never selects a winner.
- `source` is relative to the catalog file during the local milestone. Absolute
  paths, `..`, symlinks, device paths, and remote URLs are rejected.
- `sha256` and `size` are checked before unpacking. A hash proves integrity
  against this catalog, not publisher identity.
- Catalog permissions are a review preview. The installed manifest must match
  them exactly before consent is requested.

The catalog is data, never a script. Unknown required fields reject the entry;
one invalid entry does not hide valid entries and never loads plugin code.

## Command surface

| Command | First-milestone behavior |
| --- | --- |
| `plugins catalog <path>` | Validate a local catalog and print its identity, entry count, and diagnostics |
| `plugins search [query]` | Search ID, name, and description in the selected local catalog |
| `plugins install <id> [--version <semver>]` | Verify and stage a local artifact; never enable it |
| `plugins update <id>` | Show the compatible candidate and permission diff, then require confirmation |
| `plugins remove <id> [--version <semver>]` | Remove only a managed, inactive installed version |

Every mutating command first prints the exact plugin ID, version, source,
permissions, and destination. Non-interactive use requires an explicit future
confirmation flag; absence of a TTY never implies consent.

`plugins list` remains the installed-registry view. `plugins search` is only a
catalog view. A search result is neither installed nor enabled.

## Installation transaction

1. Parse and validate the catalog without opening an artifact.
2. Resolve one exact ID and version compatible with the current host.
3. Open the relative source without following symlinks outside the catalog
   directory; enforce compressed and expanded size/file-count limits.
4. Stream the artifact through SHA-256 verification into a temporary directory.
5. Reject absolute paths, parent traversal, links, duplicate names, device
   files, and platform-invalid names while extracting.
6. Validate the embedded manifest and require exact ID/version/permission
   agreement with the catalog entry.
7. Move the staged directory atomically into the managed plugin store when the
   platform permits, with a recoverable fallback documented per OS.
8. Leave the plugin disabled. Activation and permission consent are separate.

An interrupted operation removes only its verified temporary directory. It
never deletes an existing working version.

## Update and removal

An update installs a new version beside the old one, validates it, and shows
the permission delta. New permissions invalidate previous consent. Switching
the active version happens only after explicit approval, and the previous
version remains available for rollback.

Removal accepts a registry identity, not a free-form path. It refuses an active
version until it is disabled and removes only the canonical managed directory.
Configuration is preserved by default so reinstalling does not silently lose
user settings; a separate explicit purge may be designed later.

## Security risks

- A compromised catalog can point to malicious but hash-matching code.
- A mutable Git tag or release URL is not an immutable identity.
- Archives can attempt traversal, link escapes, decompression bombs, duplicate
  entries, or Windows reserved names.
- Updates can expand permissions or lower compatibility requirements.
- Removal can become arbitrary deletion if it accepts paths.
- Network catalogs introduce TLS, availability, privacy, revocation, signing,
  and publisher-identity problems that a local hash does not solve.

The local milestone addresses integrity and safe file handling, not publisher
trust. No plugin runtime should be enabled until the permission, compatibility,
supervision, and isolation work is complete.

## Remote marketplace boundary

A later opt-in remote catalog may use immutable HTTPS artifacts and signed
metadata after a dedicated threat model and key-rotation design. It must be
disabled by default, cache data with an expiry, disclose network access, and
continue to support fully local catalogs. Ratings, accounts, payments,
telemetry, automatic publishing, and a hosted gallery are outside this project.

## Implementation slices

1. Catalog DTO, parser, diagnostics, and path-safe local source resolution.
2. Read-only `catalog` and `search` commands.
3. Bounded archive verifier and staging transaction.
4. Local install with no activation.
5. Permission-aware update and managed removal.
6. Only then, a separate decision on signed remote metadata.

Each slice needs fixtures for paths with spaces, LF/CRLF, invalid UTF-8 file
names where supported, traversal attempts, partial writes, and Windows/macOS/
Linux behavior.
