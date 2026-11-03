# MONOcc for macOS

A native SwiftUI profile editor backed by Sextant's device protocol. GPL-3.0-or-later; preserve the parent project's LICENSE and credits. This is an independent app for compatible Monogram hardware.

The editor follows NSWorkspace application activation, displays the foreground application's icon and/or name on the Core's 240×240 screen, stores per-app keyboard/scroll/absolute-volume bindings and individual LED colors, and learns controls by stable module UID. Assignments stay blank until the user creates them; a Finder starter can add three non-destructive shortcuts. No action executes while the editor is foreground or actions are disabled. Accessibility approval is required for shortcut/scroll execution.

Profiles are saved atomically under `~/Library/Application Support/MONOcc/profiles.json`. Runtime screen and connection logs live beside them. The native UI uses bundled `sextant tools profile-bridge` through pipes, not a network server. Device commands and screen payloads use the existing driver. No firmware modifications are made.

## Build

From the repository root on Apple Silicon macOS with Apple command line tools:

```
cargo build --release -p sextant-cli
bash platform/macos/build.sh
```

The result is `dist/MONOcc.app`. The app is locally ad-hoc signed, not notarized. Before connecting, stop BBC inspection or any other software using the device. The existing Sextant configuration must have been initialized (`sextant init`); the editor initializes bundled resources automatically if missing.

## Current limits

Keyboard shortcuts, scrolling, and Mac output volume are implemented. MIDI/DAW parameter integration and app-specific APIs are future work. Orbiter center-press semantics remain unconfirmed; only ring and X/Y pad inputs are exposed. Input coverage has been verified on nine modules. Physical grid rendering is not used by the editor. Profile colors and native app screen must still be confirmed on the user's physical hardware. macOS foreground-app tracking and Accessibility permissions require interactive confirmation; automated tests cannot establish those integrations.

Use `MONOGRAM_PROFILE_HOME=/absolute/test/path` to isolate test data. `--simulator` runs the app's connection using the bundled simulated layout. It does not pretend that real hardware is connected.


## Current editor behavior

App paths distinguish multiple installed versions of Digital Performer, even
when the bundle identifier is identical. The foreground process's bundle path
selects the appropriate profile. Existing profiles from the first Monogram
Profiles build are migrated without deleting their source file.

Identify flashes a unit amber and restores the active profile's colors.
User-defined unit names persist by hardware UID. LED colors belong to the
physical knob/slider/button and are shared by that control's event bindings.

Export backup uses a Save dialog to select any destination. It includes all
profiles and unit names. Restore merges profiles by app identity/path and
makes a local copy of current settings first.

The Orbiter pad's page mode learns an UP push's dominant axis and sign.
A thresholded push sends one native Page Up or Page Down key, with a latch
that re-arms only after centering. No repeat occurs while held. This uses
standard Mac page keys; actual page handling depends on the foreground app
and focused content. Ring scroll has paired reverse direction, and scroll
events target the active app's focused window rather than the mouse position.

Run `bash platform/macos/test.sh` to check profile persistence, distinct DP
versions, stable module identity, independent assignments, colors, ring
pairing, calibrated page actions, neutral re-arming, backups, and screen modes.
The local verification expects the user's installed Affinity Publisher and
Digital Performer versions; portable CI fixtures are future work.


## 0.1.2 changes

Copy settings from another profile copies assignments, LED choices and screen
style, retaining the destination's identity/path and saving an automatic
before-copy backup. Native tests check that later edits are independent.

Page input activation uses a gentler threshold (30) with a lower center
threshold (10) for hysteresis. Native page/scroll events explicitly clear
modifier flags and enter the standard HID event path. Scroll location targets
the focused application window. Connection logs now show action gates and
sent page/scroll commands. The Enable actions preference persists across
restarts. Live browser verification remains necessary; simulated checks alone
do not prove app-specific event handling.


## 0.1.3 fixes

Only one calibrated page axis is permitted per Orbiter. Startup repairs a
conflicting second page binding after backing up existing profiles, preferring
the calibrated binding's label. Both axes must be centered to re-arm the shared
pad latch. Manual page assignments are also mutually exclusive.

Every connected physical light receives an explicit color, including white
for unset controls. Colors are reapplied after startup settles; a manual Apply
profile colors action is available. The native checks assert all 23 lights in
the nine-unit setup receive white defaults. This verifies commands, not physical
LED calibration. App screen updates now bypass the unchanged internal-title
shortcut; integration tests verify two successive app-frame changes.

The editor restores the last selected profile and shows assignment counts.
Unreadable profile files are never overwritten by default settings; saving is
paused until a backup is restored. Native tests verify preservation of an
intentionally malformed settings file.

## MONOcc 0.1.10

Rotate Console saves a global 0/90/180/270-degree orientation and reapplies it
when connecting. It uses the driver's layout rotation and reuploads the active
screen and profile LEDs afterward. Recalibrate Orbiter page direction after
physically turning the console. Slider label order remains separately adjustable.

Reset Module, in each unit's ellipsis menu, clears only that UID's assignments
and LED colors in the selected profile. It confirms scope and writes a complete
backup before changing saved profiles. Names and white balance remain intact.

Customize Program Icon embeds a resized PNG in the selected profile for its Core
screen and profile list. Original application files are untouched. Icons are
included in export/restore and profile copy; older profiles remain readable.

## MONOcc 0.1.12 — background controller

The controller engine is owned by an AppKit application delegate, independent of
its optional SwiftUI setup window. Closing setup keeps foreground-app tracking,
Core screen updates, LEDs and assigned actions running. Closing also ends temporary
learning/calibration/identification/light-preview modes. A menu bar item reopens
setup or explicitly stops MONOcc. Quit stops the background engine; window Close
does not. Reopening the app returns to the same engine and settings.

A bundled login helper uses SMAppService.loginItem and opens the parent app with
--background without activating it or showing setup. First launch from /Applications
attempts to enable login startup; the checkbox and menu reflect actual system
status. A user-disabled item is not re-enabled on every launch. Approval, if macOS
requires it, is linked to Login Items settings. This is a per-user GUI-session
background app, retaining the existing app identity for Accessibility, not a root
LaunchDaemon or a second independent owner of profiles/USB. No new Accessibility
rights are granted by the app.

An exclusive file lock prevents duplicate controller engines. Explicit Disconnect
suppresses retry; unexpected helper exit retries after five seconds. A stuck initial
connection is terminated after twenty seconds. The bridge reports USB shutdown
instead of continuing to emit a cached layout; reconnect restores the saved global
orientation, active profile screen and LED plan. Physical unplug/replug and login
startup require validation on the installed app.

## MONOcc 0.1.13 — pad page release

Page mode rearms from the calibrated page axis alone, so drift on the perpendicular
axis cannot keep a released vertical push latched. At sensitivity 1, release is
within 22.5 units (formerly both axes had to be within 10), and activation remains
over 30 units. Sensitivity now changes activation using 15 + 15/gain, with gain
bounded to the UI range 0.25–4. Release is the smaller of 24 or 75% of activation,
so there is always hysteresis. No timed cooldown was added. Holding above release
continues to produce only one page command; direction reversal requires release.
Saved mappings, calibration and sensitivity values are preserved.
