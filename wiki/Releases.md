# Releases

`main` contains a linear sequence with one commit per published version.
Ongoing work and detailed contribution history remain on `develop`. Current
release history:

- `V0.0.4`
- `V0.1.0`
- `v0.2.0`
- `v0.3.0`
- `v1.0.0-alpha.1` — prerelease
- `v1.1.0`
- `v1.2.0`
- `v1.2.1`
- `v1.2.2`
- `v1.3.0`
- `v1.4.0`
- `v1.5.0`
- `v1.6.0` — latest stable

Releases provide Windows and Linux archives plus native desktop installers, and
a macOS archive built by the release workflow. Each asset is covered by
`SHA256SUMS.txt`; archives include the MIT license and third-party notices.
Packages are currently unsigned. The macOS package has not been validated on
Apple hardware.

Release tags are immutable publication points after verification. A local build
proves only its host platform. See the
[release procedure](https://github.com/Blazorde/terminal_rust/blob/main/docs/release.md)
and [desktop validation guide](https://github.com/Blazorde/terminal_rust/blob/main/docs/development/desktop.md).
