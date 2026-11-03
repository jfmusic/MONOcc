# MONOcc Linux starting point

This is a contributor preview for **Ubuntu 24.04 on Intel/AMD 64-bit, using an
X11/Xorg session**. It is not a finished public release or a complete Mac port.

Open the supplied `.deb` with your package installer, then launch **MONOcc
Preview**. Reconnect the Core after installation so the USB permission rule can
apply. Use **Use last active app** to create a profile, select a button and assign
**Ctrl+A**, then try it in a text editor. Close setup and verify the button still
works. Reopen it from the applications menu if the tray icon is hidden.

Implemented as a starting point: CoreV2 connection, common shortcuts, ring
scrolling, calibrated pad pages, LED colors, backups, module reset and background
startup. **Not yet implemented:** native Wayland control, Core screen icons and
rotation, arbitrary shortcut recording, audio volume and complete setup polish.

Build and shared logic checks passed on the Mac. **Linux installation, interface
behavior, USB permissions and hardware have not been tested yet.** Please report
the distro/session, module list, connection result and the first failed step.
Profiles and logs are available from **Open profiles & log folder**. Do not send
personal profiles or documents unless needed and reviewed.

The source package includes build instructions and a list of remaining work so
other open-source developers can continue the port. No package has been uploaded
to a repository or sent to a tester automatically.
