//! Native profile editor transport. Decoded input lines are emitted by inspect.
use crate::common::{run_inspect, Globals};
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub fn run(globals: &Globals) -> Result<()> {
    run_inspect(globals, |c| {
        c.wait_for_layout();
        let stdout = std::io::stdout();
        let emit_layout = || {
            let modules: Vec<Value> = c
                .render_components()
                .iter()
                .map(|m| {
                    json!({
                        "index": m.index, "uid": m.uid, "kind": m.kind, "name": m.kind_name
                    })
                })
                .collect();
            println!("{}", json!({"modules": modules}));
            let _ = stdout.lock().flush();
        };
        emit_layout();
        for line in std::io::stdin().lock().lines() {
            let line = line?;
            if c.is_shutdown() {
                println!("{}", json!({"disconnected":true}));
                let _ = stdout.lock().flush();
                break;
            }
            let command: Value = match serde_json::from_str(&line) {
                Ok(c) => c,
                Err(e) => {
                    println!("{}", json!({"error":e.to_string()}));
                    continue;
                }
            };
            if command.get("quit").is_some() {
                break;
            }
            if let Some(degrees) = command.get("orientation").and_then(Value::as_i64) {
                if [0, 90, 180, 270].contains(&degrees) {
                    c.rotate(degrees as i32 - c.layout_orientation());
                    println!("{}", json!({"orientationApplied": degrees}));
                } else {
                    println!("{}", json!({"error": "Orientation must be 0, 90, 180 or 270 degrees"}));
                }
            }
            if let Some(path) = command.get("frame").and_then(Value::as_str) {
                match std::fs::read(path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| c.app_display_frame(b))
                {
                    Ok(()) => println!("{}", json!({"screenQueued": true})),
                    Err(e) => println!("{}", json!({"error": e})),
                }
            }
            if let Some(leds) = command.get("leds").and_then(Value::as_array) {
                let mut colors = Vec::new();
                for led in leds {
                    if let (Some(uid), Some(sub), Some(rgb)) = (
                        led.get("uid").and_then(Value::as_str),
                        led.get("sub").and_then(Value::as_u64),
                        led.get("rgb").and_then(Value::as_array),
                    ) {
                        if sub > 2 || rgb.len() != 3 {
                            continue;
                        }
                        let values: Option<Vec<u8>> = rgb
                            .iter()
                            .map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
                            .collect();
                        if let Some(v) = values {
                            colors.push((uid.to_string(), sub as usize, (v[0], v[1], v[2])));
                        }
                    }
                }
                c.app_led_colors(colors);
            }
            emit_layout();
        }
        Ok(())
    })
}
