# MONOcc Linux — contributor starting point

Version 0.2.0 preview. Target: Ubuntu 24.04 amd64, with an X11/Xorg desktop
session and Monogram CoreV2. This is an initial port, not a finished public Linux
release. It compiles into a self-contained Linux ELF application. It has not been
run or hardware-tested on Linux in this session.

## Implemented starting point

- Avalonia setup UI and background process independent of the window.
- Tray menu, application-launcher reopening through a per-user pipe, single
  instance, optional XDG desktop autostart and explicit Stop.
- USB CDC serial detection restricted to CoreV2 VID/PID, framed MessagePack,
  layout and module inputs, reconnect, saved LED choices.
- Profiles selected by the foreground X11 application's executable; common
  shortcuts, ring scrolling, calibrated page actions, reversal and sensitivity.
- Platform-tagged Linux backups, atomic persistence and scoped module reset
  with a backup first. Shared models/protocol live in ../windows/Monocc.Core.
- Debian package with launcher, icon and a targeted udev uaccess rule. No world
  writable devices or root background service. Reconnect USB after installation.

## Not implemented yet

Native Wayland app tracking/input, Core screen icons/uploads/rotation, arbitrary
shortcut recording, audio volume, full profile-management polish, Linux-specific
hardware tests and a validated distribution/update process. The setup UI states
these limits. Wayland actions are disabled rather than pretending XWayland covers
native Wayland applications. GNOME may hide tray icons; reopen via the launcher.

## Install for testing

Open MONOcc-0.2.0-Linux-amd64-preview.deb with a package installer on Ubuntu 24.04.
The package installer handles system-library dependencies and authorization.
Launch MONOcc Preview from the applications menu. No separate .NET installation
or developer tools are required for the published package. The package is not
repository-signed and has not had a clean-install test yet.

Use an X11/Xorg session. Add an app using Use last active app, then select a unit
and control and assign a common shortcut. Close setup and test in that app.
Setup can reopen from the launcher even where no tray icon is shown. Use Stop
MONOcc and Exit to release the device. Profiles stay in the current user's
XDG configuration directory under MONOcc; uninstall keeps them. A TryExec guard
prevents a retained startup entry from launching a removed binary.

## Build for contributors

Install the .NET 10 SDK and Python 3, then run platform/linux/build.sh from a
checkout. It runs shared core checks, cross-publishes linux-x64 and creates a
Debian package. No credentials or personal defaults are included. Package
metadata and post-install behavior still need validation with dpkg on Linux.
Do not confuse a successful cross-build with Linux runtime or hardware testing.

## Licensing

GPL-3.0-or-later, derived in part from sextant protocol/control work. See the root
LICENSE and accompanying third-party notices for .NET, Avalonia, SkiaSharp and
other runtime dependencies. Matching application source accompanies the preview.
Before public release, choose a real maintainer contact and distribution signing,
validate all dependencies and permission behavior on a clean Ubuntu installation,
and implement/test the remaining features above.
