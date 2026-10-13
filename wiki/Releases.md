# Releases

`main` contains a linear sequence with one commit per published version.
Ongoing work and detailed contribution history remain on `develop`. Current
release history:

- `V0.0.4`
- `V0.1.0`
- `v0.2.0`
- `v0.3.0` — latest stable
- `v1.0.0-alpha.1` — current prerelease

The alpha provides Windows and Linux archives plus native desktop installers.
Each asset is covered by `SHA256SUMS.txt`; archives include the MIT license and
third-party notices. Packages are currently unsigned. The macOS package remains
pending until a native build and PTY launch are validated on Apple hardware.

Release tags are immutable publication points after verification. A local build
proves only its host platform. See the
[release procedure](https://github.com/Blazorde/terminal_rust/blob/main/docs/release.md)
and [desktop validation guide](https://github.com/Blazorde/terminal_rust/blob/main/docs/development/desktop.md).
