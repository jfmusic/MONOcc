//! sextant-firmware: the failsafe-first avr firmware for the at90usb1286 base.
//!
//! this crate builds only with the nightly avr toolchain and the avr target (see
//! firmware/rust-toolchain.toml, firmware/.cargo/config.toml, and
//! firmware/README.md); it is excluded from the host workspace.
//!
//! the foremost deliverable is the never-brick failsafe. the brick-or-not
//! decision is the host-tested [`sextant_failsafe`] crate; this file is the thin
//! avr glue that reads the eeprom boot token, applies the decision (jumping to
//! the factory dfu bootloader on anything but a committed run token), arms the
//! failsafe before the application starts, and commits health only after the
//! firmware reaches steady state. usb-cdc, led pwm, and the json command set come
//! afterward (see ROADMAP.md phase 7); until usb exists, the health gate is
//! "reached steady state and survived a stable delay".

#![no_std]
#![no_main]

use panic_halt as _;

use avr_device::at90usb1286::Peripherals;
use sextant_failsafe::{boot_action, BootAction, ARMED_TOKEN, RUN_TOKEN};

/// the boot token lives in the first eeprom byte.
const EEPROM_BOOT_SLOT: u16 = 0;

/// word address of the factory atmel dfu bootloader entry. for the at90usb1286
/// with the standard 8 kb dfu bootloader, BOOTLOADER_START = FLASHEND - 8192 + 1
/// = 0x1_E000 (byte); the avr `icall` jumps to the word address 0xF000. if a
/// device ships a different bootloader size (per the BOOTSZ fuses) this must be
/// recomputed before flashing.
const BOOTLOADER_WORD_ADDR: u16 = 0xF000;

/// cpu frequency after the prescaler is cleared (the palette base runs at 16 mhz).
const F_CPU: u32 = 16_000_000;

/// how long the firmware must run before it commits a healthy run token.
const HEALTH_GATE_MS: u16 = 2000;

#[avr_device::entry]
fn main() -> ! {
    let dp = Peripherals::take().unwrap();

    // a leftover watchdog must not reset us mid-boot, and the bootloader expects
    // it off; do this before anything else, including a possible jump.
    reset_watchdog(&dp);

    // the single never-brick decision, from the host-tested crate.
    let token = eeprom_read(&dp, EEPROM_BOOT_SLOT);
    if boot_action(token) == BootAction::JumpToDfu {
        unsafe { jump_to_bootloader() }
    }

    // arm before the application runs: from here on, a crash or hang before the
    // commit below leaves a non-run token, so the next boot recovers to dfu.
    eeprom_write(&dp, EEPROM_BOOT_SLOT, ARMED_TOKEN);

    configure_clock(&dp);
    configure_led_ports(&dp);
    post_indicate(&dp);

    // a hung firmware now auto-recovers: the watchdog resets it and the next boot
    // sees the armed token. survive the health gate before committing.
    enable_watchdog_4s(&dp);
    delay_ms(&dp, HEALTH_GATE_MS);

    eeprom_write(&dp, EEPROM_BOOT_SLOT, RUN_TOKEN); // commit health
    committed_indicate(&dp);

    loop {
        watchdog_reset(&dp);
        avr_device::asm::sleep();
    }
}

// the control registers below are written with raw bit values from the
// at90usb1286 datasheet rather than named svd fields, so the timed write
// sequences (eeprom, watchdog) are explicit and version-independent.
const EECR_EEPE: u8 = 0x02;
const EECR_EEMPE: u8 = 0x04;
const EECR_EERE: u8 = 0x01;
const MCUSR_WDRF: u8 = 0x08;
const WDTCSR_WDCE: u8 = 0x10;
const WDTCSR_WDE: u8 = 0x08;
const WDTCSR_4S: u8 = 0x20; // WDP3 set, WDP[2:0]=0 -> ~4 s timeout
const CLKPR_CLKPCE: u8 = 0x80;
const LED_RED: u8 = 0x10; // PORTC4
const LED_GREEN: u8 = 0x20; // PORTC5
const LED_BLUE: u8 = 0x40; // PORTC6
const LED_MASK: u8 = LED_RED | LED_GREEN | LED_BLUE;

// -- eeprom (the boot token) --

/// read one eeprom byte, waiting for any in-progress write to finish.
fn eeprom_read(dp: &Peripherals, address: u16) -> u8 {
    while dp.EEPROM.eecr().read().bits() & EECR_EEPE != 0 {}
    dp.EEPROM.eear().write(|w| unsafe { w.bits(address) });
    dp.EEPROM.eecr().write(|w| unsafe { w.bits(EECR_EERE) });
    dp.EEPROM.eedr().read().bits()
}

/// write one eeprom byte using the required two-step EEMPE then EEPE sequence.
fn eeprom_write(dp: &Peripherals, address: u16, data: u8) {
    while dp.EEPROM.eecr().read().bits() & EECR_EEPE != 0 {}
    dp.EEPROM.eear().write(|w| unsafe { w.bits(address) });
    dp.EEPROM.eedr().write(|w| unsafe { w.bits(data) });
    dp.EEPROM.eecr().write(|w| unsafe { w.bits(EECR_EEMPE) });
    // set EEPE within four cycles, preserving EEMPE (the sbi-equivalent).
    dp.EEPROM
        .eecr()
        .modify(|r, w| unsafe { w.bits(r.bits() | EECR_EEPE) });
}

// -- watchdog --

/// clear the watchdog reset flag and disable the watchdog (timed sequence).
fn reset_watchdog(dp: &Peripherals) {
    let mcusr = dp.CPU.mcusr().read().bits();
    dp.CPU.mcusr().write(|w| unsafe { w.bits(mcusr & !MCUSR_WDRF) });
    dp.WDT
        .wdtcsr()
        .write(|w| unsafe { w.bits(WDTCSR_WDCE | WDTCSR_WDE) });
    dp.WDT.wdtcsr().write(|w| unsafe { w.bits(0) });
}

/// enable the watchdog with a ~4 second timeout so a hang auto-recovers to dfu.
fn enable_watchdog_4s(dp: &Peripherals) {
    watchdog_reset(dp);
    dp.WDT
        .wdtcsr()
        .write(|w| unsafe { w.bits(WDTCSR_WDCE | WDTCSR_WDE) });
    dp.WDT
        .wdtcsr()
        .write(|w| unsafe { w.bits(WDTCSR_WDE | WDTCSR_4S) });
}

fn watchdog_reset(_dp: &Peripherals) {
    avr_device::asm::wdr();
}

// -- clock, ports, and indication --

/// clear the clock prescaler so the cpu runs at the full 16 mhz.
fn configure_clock(dp: &Peripherals) {
    dp.CPU.clkpr().write(|w| unsafe { w.bits(CLKPR_CLKPCE) });
    dp.CPU.clkpr().write(|w| unsafe { w.bits(0) });
}

/// the base rgb led is on port c bits 4 (red), 5 (green), 6 (blue).
fn configure_led_ports(dp: &Peripherals) {
    dp.PORTC
        .ddrc()
        .modify(|r, w| unsafe { w.bits(r.bits() | LED_MASK) });
}

fn set_leds(dp: &Peripherals, red: bool, green: bool, blue: bool) {
    let mut value = dp.PORTC.portc().read().bits() & !LED_MASK;
    if red {
        value |= LED_RED;
    }
    if green {
        value |= LED_GREEN;
    }
    if blue {
        value |= LED_BLUE;
    }
    dp.PORTC.portc().write(|w| unsafe { w.bits(value) });
}

/// power-on self test: a brief white flash showing the firmware reached init.
fn post_indicate(dp: &Peripherals) {
    set_leds(dp, true, true, true);
    delay_ms(dp, 250);
    set_leds(dp, false, false, false);
}

/// a green flash once the run token is committed (the firmware is trusted).
fn committed_indicate(dp: &Peripherals) {
    set_leds(dp, false, true, false);
    delay_ms(dp, 250);
    set_leds(dp, false, false, false);
}

/// busy-wait, kicking the watchdog so long gates do not trip it.
fn delay_ms(dp: &Peripherals, ms: u16) {
    let cycles_per_ms = F_CPU / 1000;
    for _ in 0..ms {
        watchdog_reset(dp);
        avr_device::asm::delay_cycles(cycles_per_ms);
    }
}

// -- the never-brick jump --

/// jump to the factory dfu bootloader. on avr a code pointer is a word address,
/// so the bootloader word address is transmuted to a function pointer and called
/// (an `icall`), matching the upstream c `((void(*)())(BOOTLOADER_START/2))()`.
unsafe fn jump_to_bootloader() -> ! {
    let bootloader: extern "C" fn() -> ! = core::mem::transmute(BOOTLOADER_WORD_ADDR);
    bootloader()
}
