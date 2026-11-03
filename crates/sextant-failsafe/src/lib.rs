//! the never-brick boot decision, as pure `no_std` logic.
//!
//! a single eeprom byte (the boot token) decides, at every power-on, whether the
//! firmware is trusted to run or whether the device should fall back to the
//! factory dfu bootloader, which is always recoverable over usb. the rule is
//! deliberately strict: *only* an exact run token runs the firmware; every other
//! value -- including the `0xff` / `0x00` an erase leaves behind -- routes to the
//! bootloader.
//!
//! the lifecycle that makes this never-brick:
//!
//! 1. at boot, read the token and call [`boot_action`].
//! 2. on [`BootAction::JumpToDfu`], jump to the bootloader (recoverable).
//! 3. on [`BootAction::ArmAndRun`], first write [`ARMED_TOKEN`] (so a crash or a
//!    bad flash before step 5 lands back in the bootloader on the next boot),
//!    then bring the application up.
//! 4. wait for a health gate (usb enumerated and stable).
//! 5. only then write [`RUN_TOKEN`] to commit, so a clean reboot runs again.
//!
//! arming *before* the application starts and committing *only after* it proves
//! healthy is the whole guarantee: a firmware that cannot reach the health gate
//! never persists a run token, so the next boot always recovers to dfu. this is
//! the fix for the upstream c firmware, which armed but never committed (so it
//! ran once per flash) -- here the decision is isolated and unit tested because a
//! mistake in it bricks hardware.

#![cfg_attr(not(test), no_std)]

/// the boot token meaning "the previous run committed health; run the firmware".
/// `128` is chosen so a blank (`0x00`) or erased (`0xff`) eeprom never matches.
pub const RUN_TOKEN: u8 = 128;

/// the token written before the application starts. any value other than
/// [`RUN_TOKEN`] routes to the bootloader, so this only needs to differ from it;
/// `0` matches the upstream firmware.
pub const ARMED_TOKEN: u8 = 0;

/// what to do at power-on, decided solely from the current boot token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootAction {
    /// jump to the factory dfu bootloader (recoverable over usb).
    JumpToDfu,
    /// arm the failsafe, then run the application.
    ArmAndRun,
}

/// the never-brick decision: run the firmware only for an exact [`RUN_TOKEN`].
pub const fn boot_action(token: u8) -> BootAction {
    if token == RUN_TOKEN {
        BootAction::ArmAndRun
    } else {
        BootAction::JumpToDfu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// arming must route to dfu, or a crash after arming would still run a
    /// broken firmware -- defeating the whole failsafe.
    #[test]
    fn arming_routes_to_dfu() {
        assert_ne!(RUN_TOKEN, ARMED_TOKEN);
        assert_eq!(BootAction::JumpToDfu, boot_action(ARMED_TOKEN));
    }

    #[test]
    fn only_the_run_token_runs_firmware() {
        assert_eq!(BootAction::ArmAndRun, boot_action(RUN_TOKEN));
        for token in 0u8..=255 {
            if token != RUN_TOKEN {
                assert_eq!(BootAction::JumpToDfu, boot_action(token), "token {token}");
            }
        }
    }

    #[test]
    fn erased_eeprom_routes_to_dfu() {
        // a dfu erase leaves 0xff; a blank cell reads 0x00. both must recover.
        assert_eq!(BootAction::JumpToDfu, boot_action(0xff));
        assert_eq!(BootAction::JumpToDfu, boot_action(0x00));
    }

    /// model the eeprom byte through the boot lifecycle and assert that only a
    /// run that reaches the commit survives a reboot; everything else recovers.
    #[derive(Clone, Copy)]
    struct Device {
        token: u8,
    }

    impl Device {
        /// returns true if the application ran (false if it jumped to dfu).
        fn boot(&mut self, reaches_health_gate: bool) -> bool {
            match boot_action(self.token) {
                BootAction::JumpToDfu => false,
                BootAction::ArmAndRun => {
                    self.token = ARMED_TOKEN; // arm before running
                    if reaches_health_gate {
                        self.token = RUN_TOKEN; // commit after health
                    }
                    true
                }
            }
        }
    }

    #[test]
    fn healthy_firmware_persists_across_reboots() {
        // freshly flashed with the committed run token (the .eep default).
        let mut device = Device { token: RUN_TOKEN };
        assert!(device.boot(true));
        assert!(device.boot(true)); // still runs after a clean reboot
        assert_eq!(RUN_TOKEN, device.token);
    }

    #[test]
    fn crash_before_commit_recovers_to_dfu() {
        let mut device = Device { token: RUN_TOKEN };
        assert!(device.boot(false)); // ran, but crashed before the health gate
        assert!(!device.boot(true)); // next boot recovers to dfu
        assert!(!device.boot(true)); // and stays recoverable until reflashed
    }

    #[test]
    fn freshly_erased_device_recovers_to_dfu() {
        let mut device = Device { token: 0xff };
        assert!(!device.boot(true));
    }
}
