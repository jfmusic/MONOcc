//! `probe`: ratchet the bulk-write traffic against a real device to find what
//! makes its core watchdog-reset (re-enumerate), so the write rate can be tuned
//! per device class. it brings the device up bare -- control handshake then a
//! single `start` -- with no configure burst and no echo, which is also the clean
//! no-echo baseline: if a bare connect never re-enumerates, the resets come from
//! our write traffic, not the device.
//!
//! phase 1 ratchets the *rate* of tiny `check` writes. phase 2 (palette) ratchets
//! the real *payload* -- the 8kb icon upload and a trailing burst -- since a small
//! message may be too cheap to trip the core even back to back.

use crate::common::{open_usb_device, Globals};
use anyhow::Result;
use serde_json::{json, Value};
use sextant_core::device::{self, control_handshake, Device, DeviceClass, DeviceHandle};
use sextant_core::message::{self, MessageFormat};
use sextant_core::resources;
use std::thread;
use std::time::{Duration, Instant};

const DRAIN_TIMEOUT: Duration = Duration::from_millis(150);
const READ_TIMEOUT: Duration = Duration::from_millis(100);
const WATCH_WINDOW: Duration = Duration::from_millis(1500);
const ICON_INDEX: i64 = 23;
// a churny device (left re-enumerating by prior resets) confounds the test, so
// wait for the layout stream to go quiet this long before testing.
const SETTLE_QUIET: Duration = Duration::from_secs(4);
const SETTLE_TIMEOUT: Duration = Duration::from_secs(45);
// phase 1 (sanity): confirm tiny writes alone do not reset it.
const RATE_SCHEDULE: &[(usize, u64)] = &[(120, 0)];
// the proven-safe single-transfer size; the split test chunks the upload to this.
const SAFE_CHUNK: usize = 2048;

pub fn run(globals: &Globals) -> Result<()> {
    let mut device = open_usb_device(globals)?;
    let class = device.class();
    let format = wire_format(class);
    let handle = device.open()?;
    let handle = handle.as_ref();
    println!("probe: {class:?} ({format:?}) -- bare connect, no configure, no echo");

    drain(handle);
    control_handshake(handle)?;
    kick_layout(handle, format);
    if !wait_until_settled(handle, format) {
        println!("device is still re-enumerating from prior resets -- power-cycle (unplug/replug) the palette and re-run the probe.");
        return Ok(());
    }
    println!("device settled (rested, no re-enumeration)");

    let check = message::encode(&[json!({"check": 1})], format, &[]);

    println!("phase 1: ratchet the rate of tiny `check` writes");
    for &(count, gap_ms) in RATE_SCHEDULE {
        let gap = Duration::from_millis(gap_ms);
        let label = format!("burst={count:>3} gap={gap_ms:>2}ms");
        let trip = round(handle, format, &label, || {
            for _ in 0..count {
                let _ = handle.bulk_write(device::ENDPOINT_BULK_OUT, &check);
                if !gap.is_zero() {
                    thread::sleep(gap);
                }
            }
        });
        if trip {
            return report(class, &label);
        }
    }

    if format == MessageFormat::Json {
        let icon = resources::asset_bytes(ICON_INDEX as u32, "raw");
        let icon = icon.as_deref().unwrap_or(&[]);
        let upload = message::encode(&[json!({"screen_write": ICON_INDEX})], format, icon);
        let pieces = upload.len().div_ceil(SAFE_CHUNK);
        println!(
            "phase 2: does splitting the {}B icon upload into <={SAFE_CHUNK}B transfers avoid the reset?",
            upload.len()
        );

        // the split is tested first, on the least-hammered device; the whole-in-one
        // transfer is the positive control that this session can still trigger a reset.
        let split = round(
            handle,
            format,
            &format!("split upload -> {pieces} transfers"),
            || {
                for piece in upload.chunks(SAFE_CHUNK) {
                    let _ = handle.bulk_write(device::ENDPOINT_BULK_OUT, piece);
                }
            },
        );
        let whole = round(
            handle,
            format,
            &format!("control: one {}B transfer", upload.len()),
            || {
                let _ = handle.bulk_write(device::ENDPOINT_BULK_OUT, &upload);
            },
        );

        println!();
        match (split, whole) {
            (false, true) => println!(
                "=> chunking works at the transfer level. visually confirm the icon rendered; \
                 if so, the fix is to split transfers >{SAFE_CHUNK}B in bulk_transfer."
            ),
            (true, _) => println!(
                "=> split still resets: the firmware reassembles the bulk stream, so the ~8KB \
                 screen_write MESSAGE is the limit, not the usb transfer size. palette \
                 screen_write has no offset field, so the image cannot be split across messages."
            ),
            (false, false) => println!(
                "=> control did not reset -- device is marginal/churny this run; re-run rested \
                 before trusting the split result."
            ),
        }
    }
    Ok(())
}

/// drain, run one write pattern, then watch for a re-enumeration; report and
/// return whether the core reset.
fn round(
    handle: &dyn DeviceHandle,
    format: MessageFormat,
    label: &str,
    send: impl FnOnce(),
) -> bool {
    drain(handle);
    let start = Instant::now();
    send();
    let reset = count_layouts(handle, format, WATCH_WINDOW) > 0;
    println!(
        "  {label} ({:>4}ms) -> {}",
        start.elapsed().as_millis(),
        if reset { "RESET (re-enumerated)" } else { "ok" }
    );
    reset
}

fn report(class: DeviceClass, label: &str) -> Result<()> {
    println!("threshold: {class:?} resets at -> {label}");
    Ok(())
}

/// wait until the device stops re-enumerating (no layout for SETTLE_QUIET), so a
/// device left churning by prior resets does not confound the test.
fn wait_until_settled(handle: &dyn DeviceHandle, format: MessageFormat) -> bool {
    let give_up = Instant::now() + SETTLE_TIMEOUT;
    while Instant::now() < give_up {
        if count_layouts(handle, format, SETTLE_QUIET) == 0 {
            return true;
        }
    }
    false
}

fn wire_format(class: DeviceClass) -> MessageFormat {
    if class == DeviceClass::PaletteGear {
        MessageFormat::Json
    } else {
        MessageFormat::Msgpack
    }
}

/// elicit the device's layout so a later re-enumeration is observable.
fn kick_layout(handle: &dyn DeviceHandle, format: MessageFormat) {
    let message = if format == MessageFormat::Json {
        json!({"start": 1})
    } else {
        json!({"get_layout": true})
    };
    let data = message::encode(&[message], format, &[]);
    let _ = handle.bulk_write(device::ENDPOINT_BULK_OUT, &data);
}

/// read and discard everything pending, so the next watch starts clean.
fn drain(handle: &dyn DeviceHandle) {
    while let Ok(data) = handle.bulk_read(
        device::ENDPOINT_BULK_IN,
        device::BULK_READ_SIZE_BYTES,
        DRAIN_TIMEOUT,
    ) {
        if data.is_empty() {
            break;
        }
    }
}

/// count the layout (`l`) messages arriving within `window`; a layout after a
/// drained burst means the core re-enumerated, i.e. it reset.
fn count_layouts(handle: &dyn DeviceHandle, format: MessageFormat, window: Duration) -> usize {
    let deadline = Instant::now() + window;
    let mut buffer: Vec<u8> = Vec::new();
    let mut layouts = 0;
    while Instant::now() < deadline {
        if let Ok(data) = handle.bulk_read(
            device::ENDPOINT_BULK_IN,
            device::BULK_READ_SIZE_BYTES,
            READ_TIMEOUT,
        ) {
            buffer.extend_from_slice(&data);
            let (messages, remainder) = message::decode(&buffer, format);
            buffer = remainder;
            layouts += messages
                .iter()
                .filter(|m: &&Value| m.get("l").is_some())
                .count();
        }
    }
    layouts
}
