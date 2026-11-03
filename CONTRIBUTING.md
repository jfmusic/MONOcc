# Contributing to MONOcc

Start with the root README platform matrix and the platform-specific README.
Open an issue or write a short test report describing the OS/session, hardware
modules, expected behavior and observed result. Never attach a user's personal
profiles or document contents without reviewing them first.

Keep module identity based on stable UID, not layout index. Preserve settings
on errors; take a backup before a reset/import/removal. Keep exactly one hardware
owner per user, and distinguish closing setup from deliberately stopping the
background controller. Global input must follow the foreground app and must not
run while setup is focused. Retain explicit direction calibration and one-page-
per-push behavior for the pad.

Windows, macOS and Linux keyboard codes and app paths are different. Do not
silently reinterpret another platform's backups. Do not replace a user's USB
driver or weaken system security as a troubleshooting shortcut.

For a hardware change, add focused logic checks, then test the real modules and
record which checks require native OS verification. For a UI/packaging change,
run the native smoke check and the install/upgrade/uninstall flow. The Windows
and Linux workflows are manual-only starting points, not evidence that those
flows have already passed.

Please keep public release changes separate from development work. Signing keys,
maintainer contact, update hosting and public distribution are not configured
in this source bundle. Preserve GPL provenance and package third-party notices.
