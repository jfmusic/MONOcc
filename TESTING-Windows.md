# MONOcc Windows preview — tester guide

**Version:** 0.2.0 testing preview  
**Target:** Windows 11 on an Intel or AMD 64-bit computer  
**Hardware:** Monogram CoreV2 and attached modules

This first Windows build is for testing, not public release. It is unsigned. The
code builds successfully and 40 shared logic checks passed. Installation, the
Windows interface, startup and the physical controller have not yet been tested
on Windows. Do not disable Windows security protections to run it. If it is
blocked, stop and report the message.

## Install

1. Extract the tester ZIP to a folder on the Windows computer.
2. Double-click **MONOcc-0.2.0-Windows-x64-Setup.exe**.
3. Choose whether to start MONOcc when you sign in, then choose **Install MONOcc**.

The installer includes the runtime: no separate .NET download, administrator
account, Terminal or developer tools are needed. It creates Start menu entries
and an uninstall entry in Windows Settings. It does not change USB drivers.

## First controller test

1. Close original Creator and any other controller software. Connect the
   Monogram Core by USB and wait for MONOcc to show connected units.
2. If it stays at **Waiting for controller**, open **Settings & Help**. Check
   Device Manager for the Monogram USB/COM device. Do not change drivers yet.
   Report what Windows shows. The serial connection is the main unknown in this
   first preview.
3. Open Windows Notepad and type a few lines. Return to MONOcc and choose
   **Use last active app** to create its profile.
4. Select a button, choose **Keyboard shortcut**, click **Record shortcut**, and
   press **Ctrl+A**. Return to Notepad and press that hardware button. It should
   select the text once.

## Background operation

Close the setup window using its **X**. The controller should keep working in
Notepad. Reopen setup from Start or the tray icon, which may be under the taskbar
**^** arrow. **Stop MONOcc and Exit** deliberately stops the controller.

Unplug and reconnect the Core. MONOcc should reconnect and restore the active
profile. If login startup is enabled, save your work and sign out and back in.
Open Notepad without manually opening MONOcc and repeat the button test.

## Additional checks — one at a time

- **Orbiter pad:** select the Orbiter and click **Set pad up/down by page**. Push
  up once, release, then test Page Up/Down in a long document. One page per push;
  holding should not repeat. Reverse only the assigned page axis. Higher
  sensitivity needs a lighter push.
- **Ring:** choose **Set ring to up/down scrolling**. Reversing either Scroll
  direction should reverse both together.
- **Screen:** try a custom profile icon and console rotation.
- **Lights:** choose a saved color for a control, then return to the app. End
  temporary light tests by choosing **Follow active app** or closing setup.
- **Reset Module:** create test assignments and colors, then reset that module.
  Other modules and profiles should remain unchanged. A backup is saved first.
- **Backups:** export a Windows backup, change a test setting, and restore it.
  Mac backups are not compatible with Windows key codes or app paths.
- **Uninstall:** stop MONOcc from the tray, then use Windows **Installed apps**
  or **Uninstall MONOcc** in Start. Profiles and backups should be retained.

## Send back results

Please report:

- Windows version and Intel/AMD or ARM processor.
- Whether installation and initial controller detection worked.
- Which physical modules were connected.
- Which test passed or failed, with a short description or screenshot.
- If connection failed: the device name/COM port shown in Device Manager and
  `connection.log` from **Settings & Help → Open settings & backups folder**.

No need to send personal profiles, documents, license keys or account details.

## Parallels

Parallels on an Apple Silicon Mac can provide an additional Windows 11 ARM test.
Assign the Monogram USB device to Windows, and stop Mac MONOcc first so only one
system owns it. This can help test setup and USB behavior, but it does not replace
an Intel/AMD Windows test before public release. USB driver compatibility remains
a separate check.

Parallels reference: [Windows on ARM compatibility](https://www.parallels.com/blogs/windows-11-on-apple-silicon/).
