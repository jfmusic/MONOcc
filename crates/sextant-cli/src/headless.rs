//! the headless scriptable mode driving the controller against the fake device.
//!
//! it reads whitespace-delimited commands on stdin (`pressed 6`,
//! `rotation_left 2 8`, `sleep 0.25`, `dump`, ...). `dump` prints one json line
//! of device + controller state. the python end-to-end suite drives this
//! protocol and asserts on the dump, reproducing the upstream `test_ambit.py`
//! behavioral contract without hardware.

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use std::io::{BufRead, Write};
use std::time::Duration;

use sextant_core::Controller;
use sextant_fake::FakeDevice;

use crate::common::{build_config, fake_device, layout_by_name, usb_error, Globals};

pub fn run(globals: &Globals) -> Result<()> {
    let config = build_config(globals)?;
    let mut controller = Controller::new(config, fake_device(globals)?);
    controller.set_echo(globals.echo);
    controller.set_debug(globals.debug);
    controller.open().map_err(usb_error)?;
    controller.connect().map_err(usb_error)?;
    controller.wait_for_layout();

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        if !dispatch(&line?, &controller, &mut stdout)? {
            break;
        }
    }

    controller.close();
    Ok(())
}

/// execute one script command. returns false when the script should stop.
fn dispatch(
    line: &str,
    controller: &Controller<FakeDevice>,
    stdout: &mut impl Write,
) -> Result<bool> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(true);
    }
    let mut parts = line.split_whitespace();
    let command = parts.next().unwrap_or("");
    let device = controller.device();

    let index = |parts: &mut std::str::SplitWhitespace| -> Result<i64> {
        parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow!("missing index"))
    };
    let amount = |parts: &mut std::str::SplitWhitespace| -> Result<i64> {
        parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| anyhow!("missing amount"))
    };

    match command {
        "pressed" => device.input_pressed(index(&mut parts)?),
        "released" => device.input_released(index(&mut parts)?),
        "slide_up" => device.input_slide_up(index(&mut parts)?, amount(&mut parts)?),
        "slide_down" => device.input_slide_down(index(&mut parts)?, amount(&mut parts)?),
        "rotation_left" => device.input_rotation_left(index(&mut parts)?, amount(&mut parts)?),
        "rotation_right" => device.input_rotation_right(index(&mut parts)?, amount(&mut parts)?),
        "sleep" => {
            let seconds: f64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
            std::thread::sleep(Duration::from_secs_f64(seconds));
        }
        "wait" => controller.wait(),
        "wait_layout" => controller.wait_for_layout(),
        "layout_changed" => {
            let name = parts.next().unwrap_or("");
            device.layout_changed(layout_by_name(name)?);
        }
        "screen_string" => {
            let text = line.strip_prefix("screen_string").unwrap_or("").trim();
            controller.screen_string(text);
        }
        "configure_images" => controller.configure_images(),
        "map_hid" => controller.configure_hidmap(),
        "dump" => {
            writeln!(stdout, "{}", build_dump(controller))?;
            stdout.flush()?;
        }
        "quit" => return Ok(false),
        other => return Err(anyhow!("unknown command {other:?}")),
    }
    Ok(true)
}

fn build_dump(controller: &Controller<FakeDevice>) -> Value {
    let handle = controller.device().handle();

    let mut leds = Map::new();
    for (index, (r, g, b)) in handle.leds() {
        leds.insert(index.to_string(), json!([r, g, b]));
    }

    let (mut values, mut orientations, mut slots, mut flips) =
        (Map::new(), Map::new(), Map::new(), Map::new());
    for index in controller.connected_indices() {
        let key = index.to_string();
        if let Some(v) = controller.component_values(index) {
            values.insert(key.clone(), json!(v));
        }
        if let Some(o) = controller.component_orientation(index) {
            orientations.insert(key.clone(), json!(o));
        }
        if let Some((x, y)) = controller.component_slot(index) {
            slots.insert(key.clone(), json!([x, y]));
        }
        if let Some(f) = controller.component_flip(index) {
            flips.insert(key.clone(), json!(f));
        }
    }

    let all_messages = handle.all_messages();
    let mut message_counts = Map::new();
    for (key, list) in &all_messages {
        message_counts.insert(key.clone(), json!(list.len()));
    }

    let mut screen_images = Map::new();
    for (index, bytes) in handle.screen_images() {
        screen_images.insert(index.to_string(), json!(to_hex(&bytes)));
    }

    json!({
        "connected": controller.connected_count(),
        "screen_string": handle.screen_string(),
        "screen_display": handle.screen_display(),
        "leds": leds,
        "values": values,
        "orientations": orientations,
        "slots": slots,
        "flips": flips,
        "message_counts": message_counts,
        "screen_display_values": all_messages.get("screen_display").cloned().unwrap_or_default(),
        "hidmap": all_messages.get("hidmap").cloned().unwrap_or_default(),
        "screen_images": screen_images,
        "failed_writes": controller.failed_writes(),
        "dropped_leds": controller.dropped_leds(),
        "dropped_screen_strings": controller.dropped_screen_strings(),
    })
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
