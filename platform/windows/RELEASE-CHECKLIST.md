# Public-release gates

- [x] Windows x64 application and self-contained executable compile.
- [x] No personal app paths, profiles, or assignments in Windows defaults.
- [x] Core logic checks: profiles, atomic save, damaged-file protection, backup
  restore, module reset scope, page hysteresis/sensitivity, framing, layout, LEDs.
- [x] Per-user installer/uninstaller and startup registration implemented.
- [x] Matching source and license notices packaged.
- [ ] Run Windows-only UI/native layout smoke test (manual workflow supplied).
- [ ] Verify clean Windows 11 installation as a standard user, Start shortcuts,
  upgrade while stopped, startup setting and complete uninstall with data kept.
- [ ] Validate USB CDC connection on physical CoreV2, not just serial emulation.
- [ ] Verify every module kind, app switching, keyboard/scroll/page/volume actions,
  LEDs, screen upload, rotation, reset, backup/restore and replug on Windows.
- [ ] Verify background startup after sign-out/sign-in and setup reopen behavior.
- [ ] Review accessibility, high DPI, small screens and keyboard-only setup.
- [ ] Arrange publisher certificate and sign/test the final executable.
- [ ] Decide public support/contact, distribution location and update policy.

No public release, certificate purchase, remote repository or external upload has
been performed as part of creating this preview.
