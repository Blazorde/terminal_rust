# Security policy

## Supported versions

Security fixes are provided for the latest stable minor release only. The
current supported line is `0.3.x`. The `develop` branch and prerelease builds
are testing surfaces and are not supported releases.

| Version | Supported |
| --- | --- |
| `0.3.x` | Yes |
| `< 0.3` | No |
| `develop` / prereleases | No |

This table is updated when a new stable version is published.

## Report a vulnerability privately

Do not open a public issue, discussion or pull request for a suspected
vulnerability. Use GitHub's private vulnerability reporting form:

https://github.com/Blazorde/terminal_rust/security/advisories/new

Include the affected version and platform, impact, reproduction steps, and a
minimal proof of concept when it is safe to share. Remove tokens, personal
data, private paths and unrelated logs before submitting the report.

The maintainer aims to acknowledge actionable reports within seven days and
will coordinate validation, remediation and disclosure through the private
advisory. These are response targets, not guarantees or a promise that every
report is a vulnerability.

## Scope and disclosure

Reports about command execution, path handling, plugin permissions, desktop
IPC, installers and release artifacts are in scope. Dependency reports should
identify the reachable behavior when possible. Please allow time for a fix and
a supported release before publishing technical details.

Automated checks reduce risk but do not prove that the software is free from
vulnerabilities. Users should still apply normal operating-system permissions,
review third-party plugins and avoid running untrusted commands.
