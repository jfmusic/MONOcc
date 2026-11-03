# MONOcc — open-source starting point

This source tree contains the existing macOS application and new Windows/Linux
ports for contributors. It is a starting point for collaboration, not a claim
that all three platforms are equally complete or ready for public release.

| Platform | State | Where to begin |
| --- | --- | --- |
| macOS Apple Silicon | v0.1.15; user-tested controls, profiles and background setup | platform/macos/README.md |
| Windows 11 Intel/AMD | v0.2.0 preview; cross-compiled, native/hardware tests pending | platform/windows/README.md |
| Ubuntu 24.04 amd64 X11 | v0.2.0 contributor port; cross-compiled, feature gaps and native/hardware tests pending | platform/linux/README.md |

The Windows/Linux shared logic has 40 passing checks for profile storage,
backups, module reset, pad release/sensitivity, protocol framing, module identity
and LED packing. Target-platform UI, installer and hardware tests must be run
separately. Build success is not hardware validation.

## Layout

- `platform/macos`: Swift setup/background app and login helper. Uses the Rust
  `sextant tools profile-bridge` binary built from this tree.
- `platform/windows`: .NET WinForms application, per-user self-contained
  installer/uninstaller, background tray controller, tests and build script.
- `platform/windows/Monocc.Core`: shared Windows/Linux models, atomic storage,
  platform-tagged backups, pad state machine and CoreV2 wire protocol.
- `platform/linux`: Avalonia/X11 port, Debian packaging, restricted USB access
  rule and documented remaining work.
- `crates`: GPL sextant controller, hardware backends, simulator and CLI.
- `.github/workflows/windows-preview.yml` and `linux-preview.yml`: manually
  invoked build/check workflows. They have not run remotely in this task and do
  not publish a public release.

No personal profile files, connection logs, credentials, email addresses or
repository history are included. Application defaults do not include private
assignments. Do not include user captures/profiles when preparing releases.

## First contribution priorities

1. Validate the Windows package on a clean Windows 11 account and physical Core,
   particularly serial USB detection, startup, replug and uninstall.
2. Validate the Linux Debian package on Ubuntu 24.04 X11, its USB access rule,
   background lifecycle, profile activation and shortcuts.
3. Finish Linux screen output/icons/rotation, shortcut recording, audio volume
   and setup polish. Add a consent-aware Wayland integration strategy; do not
   assume XWayland grants control of native Wayland apps.
4. Refactor transport/controller behavior shared by the ports where practical,
   while retaining platform-specific shortcut codes and app identities.
5. Complete signing, maintainer identity, distribution and update policies before
   advertising a public release. The current Windows preview is unsigned.

Mac test scripts currently include assumptions about locally installed app
versions. Make test application fixtures self-contained before relying on them
in a clean public CI environment. This does not change the tested Mac app build.

## Build

See each platform README. Windows and Linux need the .NET 10 SDK for development;
self-contained end-user packages do not. The Linux packager also needs Python 3.
macOS needs Swift and the Rust toolchain. Existing Rust controller tests can be
run with `cargo test`. The shared desktop checks run via
`dotnet run --project platform/windows/Monocc.Tests -c Release`.

## License and provenance

GPL-3.0-or-later. See `LICENSE`. The controller and protocol work builds on
[sextant](https://github.com/khimaros/sextant). The upstream project README is
preserved as `README-sextant.md` in the source package. Third-party .NET/Avalonia/
Skia and serial-runtime notices are included with preview distributions.

This repository is a contributor starting point. Run the native platform checks
and hardware tests before distributing a release. Windows and Linux previews
are not yet validated public releases.
