# MONOcc for Windows — 0.2.0 testing preview

This is an unsigned first Windows preview for Windows 11 on Intel/AMD x64.
It is a test build, not a validated public release. The application and per-user
installer compile successfully. Core logic checks run on the build Mac. Windows
UI, installation, login startup, uninstallation and physical USB behavior still
require testing on Windows. A Windows-only smoke check and manual CI workflow
are supplied but have not been run in this session.

## For testers

Use the supplied setup EXE; no .NET installation or developer tools are required.
Close original Creator and other controller software before connecting the Core.
A normal installation creates Start menu entries, an Installed Apps uninstall
entry, and an optional per-user startup entry. No administrator account is used.
Profiles live in `%LOCALAPPDATA%\MONOcc` and are preserved on uninstall.

Setup can be closed without stopping the background controller. Reopen it from
Start or the MONOcc tray icon, which may be hidden under the taskbar ^ arrow.
Choose **Stop MONOcc and Exit** to stop it completely.

The test build does not have a publisher certificate. Do not disable security
protections to run it. If Windows blocks it, report the message and pause testing.

## Implemented

- Per-executable app profiles; no personal app paths or assignments included.
- Button and knob shortcuts, ring scrolling, calibrated Orbiter page pushes,
  sensitivity and direction reversal; slider output volume via Core Audio.
- Core screen icon/name/custom PNG, host-rendered console rotation, saved LED
  colors, temporary test lights, stable hardware IDs, unit names and label order.
- Module reset scoped to one selected profile with a backup first.
- Windows backup export/restore, profile rename/duplicate/remove.
- Background tray operation, reopen signaling, one controller process per user,
  reconnect retries, optional login startup, help and troubleshooting.

## USB transport — needs hardware validation

The Windows version uses `System.IO.Ports` and the Windows USB CDC serial driver,
not the macOS nusb helper. Automatic selection intersects active COM ports with
USB registry instances for VID_0483/PID_A2C5. Other devices are never opened
implicitly. The advanced COM selector is explicitly chosen by the user.

CoreV2 MessagePack framing/escaping, stable layout IDs, 8-byte register decoding,
low-byte-red LED packing, and four-row RGB565 screen strips are derived from
sextant and covered by logic checks. The serial port is configured as 300000 baud,
8N1, control lines cleared, based on the existing CDC handshake. This transport
has not yet been confirmed on a Windows-connected Monogram Core. Some existing
driver bindings may not expose a COM port. The installer does not replace drivers.
Legacy Palette Gear and DAW-specific plugins/MIDI are not included in this preview.

## Cross-platform profiles

Windows shortcuts use Windows virtual keys. Mac key codes and application paths
are incompatible; Mac backups are rejected with an explanation. This preview
accepts only backups marked `platform: "windows"` and version 1. Profiles must be
created and verified on Windows before sharing.

## Build and validate

Source requires the .NET 10 SDK. From a Windows development checkout, run
`platform/windows/build.ps1`. It runs the shared core checks, publishes a
self-contained win-x64 EXE, and runs the Windows smoke test. End users do not run
this script. The manual `windows-preview.yml` workflow performs the same checks
on a Windows runner; it does not publish a public release or sign the executable.

For a Mac cross-build, use `dotnet publish` with the Windows project, `-r win-x64`,
`--self-contained true` and an output directory. Cross-build success cannot verify
Windows UI behavior or hardware compatibility.

## Licensing and release gates

MONOcc Windows code is GPL-3.0-or-later, derived in part from the sextant GPL
controller/protocol implementation: https://github.com/khimaros/sextant .
The matching application source is supplied with the preview. .NET runtime and
serial-port dependency license notices accompany the binary package.

Before a public release: pass the clean Windows install/uninstall and hardware
checklist, verify background startup/replug behavior, review distribution/license
notices, arrange publisher code signing and test the signed installer on a clean
Windows account. No signing certificate, public hosting or release account has
been configured by this task.
