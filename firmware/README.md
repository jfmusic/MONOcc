# sextant-firmware

avr firmware for the at90usb1286 in the palette base. a rust port of
ambit-firmware, built failsafe-first: the foremost goal is a bulletproof
bootloader so a device can never be bricked during a firmware update.

status: the failsafe firmware is implemented and builds to a flashable avr
image. the never-brick *decision* is host-tested. it has NOT been verified on
hardware yet -- do that on a teensy++ 2.0 / dev board before flashing any
device you care about (see "verify before you flash" below). usb-cdc, led pwm,
and the json command set are future work (see the top-level ROADMAP.md phase 7).

## why this is a separate crate

it is `no_std`, targets 8-bit avr, and needs a nightly toolchain. it is excluded
from the host cargo workspace and built from this directory.

## toolchain

rust on avr is nightly-only. `embassy` does not support 8-bit avr, so the stack
is `avr-device` (the official svd-generated register access) plus `avr-gcc` for
linking. building also needs `avr-gcc`/`avr-objcopy` on `PATH`.

```
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
make firmware       # builds firmware/target/avr-none/release/sextant-firmware.elf
make firmware-hex   # emits firmware/target/sextant-firmware.hex
make firmware-test  # runs the host-testable never-brick decision logic
```

the target, `build-std`, linker, and `-mmcu` are configured in
`firmware/.cargo/config.toml`.

## failsafe / never-brick (the first deliverable)

the factory atmel dfu bootloader lives in the ISP-only BOOT section and performs
dfu flashing of the application (USER) section. the danger is flashing a broken
application that never returns to dfu -- recoverable only over ISP (opening the
device). the failsafe guarantees a return path to dfu.

the brick-or-not decision is isolated in the dependency-free, `no_std`
[`sextant-failsafe`](../crates/sextant-failsafe) crate so the exact logic the
firmware runs is unit tested on the host (a mistake here bricks hardware):

- an eeprom boot token (slot 0): `128` (the run token) = the previous run
  committed health, run the firmware; **anything else** -- including the `0xff` /
  `0x00` an erase leaves -- routes to the dfu bootloader.
- on boot: read the token; on anything but the run token, jump to dfu immediately
  (recoverable over usb).
- otherwise *arm* (write a non-run token) **before** bringing the app up, so a
  crash, hang, or bad flash before the commit leaves a non-run token; then run,
  and only **commit** the run token after a health gate.

arming before the app and committing only after it proves healthy is the whole
guarantee. this fixes the c original, which armed but never committed (so it ran
once per flash). until usb exists the health gate is "reached steady state and
survived a stable delay"; once usb-cdc lands it becomes "usb enumerated + stable".

the avr glue (`src/main.rs`) also clears the clock prescaler, drives the base rgb
led for a power-on self test, and enables a ~4 s watchdog so a hang auto-recovers
to dfu. register access uses the svd-generated `avr-device` pac so addresses are
not hand-typed; the dfu jump is an `icall` to the bootloader word address
(`0xF000` for the 8 kb atmel dfu -- recompute if the BOOTSZ fuses differ).

## verify before you flash

R35 makes on-hardware verification mandatory before any production flash. on a
teensy++ 2.0 (same at90usb1286), confirm, with `make firmware-hex` flashed over
dfu and the `.eep` carrying a committed run token:

1. a healthy build boots (white POST flash, then a green commit flash) and keeps
   running across clean reboots.
2. interrupting power during the health gate (before the green flash) lands the
   next boot in dfu (the led does not reach the green flash; the device
   re-enumerates as the atmel dfu device).
3. a deliberately-hanging build is auto-recovered to dfu by the watchdog.

only after all three hold should the failsafe be trusted on a real device.

## recovery of last resort

if the failsafe is ever defeated, the device is recoverable over a 6-pin ISP
header on the mainboard (see the upstream ambit-firmware HARDWARE.md).
