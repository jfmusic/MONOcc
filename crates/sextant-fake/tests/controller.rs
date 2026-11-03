//! end-to-end controller behavior against the in-memory fake device. these port
//! a representative subset of the upstream `test_ambit.py` integration tests.

use sextant_core::controller::MAX_SAFE_TRANSFER;
use sextant_core::resources;
use sextant_core::{Configuration, Controller, Led};
use sextant_fake::{layouts, FakeDevice};
use std::thread::sleep;
use std::time::Duration;

/// build a configuration from a named profile set in the embedded seed (its
/// `.plp` files), so the test needs no extracted configuration home.
fn config_from_profiles(name: &str) -> Configuration {
    let mut config = Configuration::default();
    let prefix = format!("profiles/{name}/");
    let mut paths: Vec<String> = resources::resource_paths()
        .into_iter()
        .filter(|p| p.starts_with(&prefix) && p.ends_with(".plp"))
        .collect();
    paths.sort();
    for path in paths {
        let bytes = resources::resource_bytes(&path).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        config.add_profile(text, &path).unwrap();
    }
    config.switch(0);
    config
}

fn settle() {
    sleep(Duration::from_millis(600));
}

#[test]
fn connect_reports_layout_and_default_screen() {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    assert_eq!(8, ctrl.connected_count());
    // geometry matches the phase 1 traversal oracle.
    assert_eq!(Some(270), ctrl.component_orientation(3));
    assert_eq!(Some((0, -1)), ctrl.component_slot(2));
    assert_eq!(Some(true), ctrl.component_flip(7));

    // the default profile drives the base screen (a clean black slot-23 icon)
    // and all leds to white.
    let handle = ctrl.device().handle();
    assert_eq!(Some("SEXTANT".to_string()), handle.screen_string());
    assert_eq!(
        vec![serde_json::json!(23)],
        handle.messages("screen_display")
    );

    ctrl.wait();
    let leds = handle.leds();
    for index in 1..=8 {
        assert_eq!(Some(&(255, 255, 255)), leds.get(&index), "led {index}");
    }
    assert_eq!(0, ctrl.failed_writes());
}

#[test]
fn multifunction_buttons_switch_and_color() {
    let config = config_from_profiles("multifunction-buttons");
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(config, device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    assert_eq!(Some("META".to_string()), handle.screen_string());

    // dial 2 turns left three times -> setColorGreen accumulates down from 255.
    ctrl.device().input_rotation_left(2, 8);
    ctrl.device().input_rotation_left(2, 8);
    ctrl.device().input_rotation_left(2, 16);
    settle();
    settle();
    assert_eq!(Some("Green: 231".to_string()), handle.screen_string());
    ctrl.wait();
    let leds = handle.leds();
    for index in 1..=8 {
        assert_eq!(Some(&(255, 231, 255)), leds.get(&index), "led {index}");
    }

    // button 6 switches to the previous profile, wrapping meta -> home.
    ctrl.device().input_pressed(6);
    ctrl.device().input_released(6);
    settle();
    settle();
    assert_eq!(Some("HOME".to_string()), handle.screen_string());

    // button 8 switches to the next profile, wrapping home -> meta.
    ctrl.device().input_pressed(8);
    ctrl.device().input_released(8);
    settle();
    settle();
    assert_eq!(Some("META".to_string()), handle.screen_string());

    assert_eq!(0, ctrl.failed_writes());
}

#[test]
fn layout_change_updates_connected_count() {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();
    assert_eq!(8, ctrl.connected_count());

    ctrl.device().layout_changed(layouts::prokit());
    ctrl.wait_for_layout();
    settle();
    assert_eq!(15, ctrl.connected_count());
}

// the palette core watchdog-resets on a single bulk-out transfer >= ~3072 bytes
// (probe: survives 2048B, resets at 3072B). a connect must never send a transfer
// that large -- the ~8kb icon upload belongs to the explicit `upload` command,
// not configure_display. this guards against re-introducing a connect-time upload.
#[test]
fn connect_transfers_stay_under_the_core_limit() {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();
    ctrl.wait();

    let sizes = ctrl.device().handle().transfer_sizes();
    assert!(!sizes.is_empty(), "expected bulk transfers during connect");
    let oversized: Vec<usize> = sizes
        .into_iter()
        .filter(|&s| s > MAX_SAFE_TRANSFER)
        .collect();
    assert!(
        oversized.is_empty(),
        "connect sent {} transfer(s) over the {MAX_SAFE_TRANSFER}B core limit: {oversized:?}",
        oversized.len()
    );
}

#[test]
fn slider_range_workaround_broken_firmware() {
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_version_core("1.3.1");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    // the device caps at 254, but the controller rewrites it back to 255.
    ctrl.device().input_slide_up(7, 254);
    settle();
    assert_eq!(Some([255, 0, 0, 0, 0, 0, 0, 0]), ctrl.component_values(7));
}

#[test]
fn slider_range_fixed_firmware() {
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_version_core("1.4.6136");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    ctrl.device().input_slide_up(7, 254);
    settle();
    assert_eq!(Some([254, 0, 0, 0, 0, 0, 0, 0]), ctrl.component_values(7));
    ctrl.device().input_slide_up(7, 1);
    settle();
    assert_eq!(Some([255, 0, 0, 0, 0, 0, 0, 0]), ctrl.component_values(7));
}

#[test]
fn inspect_mode_processes_input_without_disruption() {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_inspect(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    // inspect mode prints a decoded [in] line per event (to stdout) but must
    // not change how input is processed: the registers still update normally.
    ctrl.device().input_rotation_left(2, 8);
    ctrl.device().input_pressed(6);
    settle();
    assert_eq!(Some([0, 8, 0, 0, 0, 0, 0, 0]), ctrl.component_values(2));
    assert_eq!(Some([1, 0, 0, 0, 0, 0, 0, 0]), ctrl.component_values(6));
    assert_eq!(0, ctrl.failed_writes());
}

#[test]
fn monogram_screen_uploads_a_frame_on_connect() {
    // the core display uses write_display_slow strips + invoke_display, not the
    // palettegear screen commands; configure_display must drive it on connect.
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    assert!(
        !handle.messages("invoke_display").is_empty(),
        "the core screen frame was committed"
    );
    assert_eq!(0, ctrl.failed_writes());
}

#[test]
fn monogram_led_addresses_tri_dial_by_kind() {
    // a monogram (msgpack) fake; the tri-dial's multi-led handling must key off
    // the component kind, not a hardcoded index.
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    let before = handle.messages("set_module").len();
    ctrl.led(vec![
        Led {
            i: 1,
            r: 255,
            g: 0,
            b: 0,
            m: 0,
        }, // tri-dial (kind 9)
        Led {
            i: 2,
            r: 0,
            g: 255,
            b: 0,
            m: 0,
        }, // orbiter (single led)
    ]);
    ctrl.wait();

    let msgs = handle.messages("set_module");
    let new = &msgs[before..];
    // tri-dial -> four field-5 writes (three sub-leds + the clear); orbiter -> one field-2 write.
    let tri = new
        .iter()
        .filter(|m| m[0].as_i64() == Some(1) && m[1].as_i64() == Some(5))
        .count();
    let orbiter = new
        .iter()
        .filter(|m| m[0].as_i64() == Some(2) && m[1].as_i64() == Some(2))
        .count();
    assert_eq!(4, tri);
    assert_eq!(1, orbiter);
}

#[test]
fn tri_dial_set_accumulates_through_the_controller() {
    // a tri-dial knob is delta-only; the controller integrates it into an
    // absolute position exposed via render_components().accumulated (the `set`).
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    // turn knob 2 (v[2]) right twice, with the device's neutral reset between.
    ctrl.device().input(1, [0, 0, 1, 0, 0, 0, 0, 0]);
    ctrl.device().input(1, [0, 0, 0, 0, 0, 0, 0, 0]);
    ctrl.device().input(1, [0, 0, 1, 0, 0, 0, 0, 0]);
    settle();
    let tri = ctrl
        .render_components()
        .into_iter()
        .find(|c| c.index == 1)
        .unwrap();
    // two right detents * sensitivity(4); the reset between contributes nothing.
    assert_eq!(8, tri.accumulated[2]);
}

#[test]
fn echo_dial_shows_only_the_accumulated_value() {
    // a dial turn reports both a rotation delta (v[2]) and its absolute set
    // position (v[3]); the on-screen echo must show only the accumulated set
    // value, once -- not a second echo for the rotation delta.
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_echo(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    let before = handle.messages("screen_string").len();
    ctrl.device().input(2, [0, 0, 8, 42, 0, 0, 0, 0]); // dial 2: turn right, set 42
    settle();
    settle();

    let echoes: Vec<String> = handle.messages("screen_string")[before..]
        .iter()
        .filter_map(|m| m.as_str().map(str::to_string))
        .filter(|s| s.starts_with("Dial:"))
        .collect();
    assert_eq!(1, echoes.len(), "one dial echo, got {echoes:?}");
    assert!(
        echoes[0].contains("42"),
        "echo shows the set value: {}",
        echoes[0]
    );
}

#[test]
fn echo_dial_stays_silent_at_the_accumulator_limit() {
    // once the dial maxes out, v[3] stops changing so `set` no longer fires; the
    // echo must not then fall back to showing the rotation velocity.
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_echo(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    ctrl.device().input(2, [0, 0, 8, 255, 0, 0, 0, 0]); // turn to the 255 limit
    settle();
    settle();
    let before = handle.messages("screen_string").len();
    ctrl.device().input(2, [0, 0, 8, 255, 0, 0, 0, 0]); // keep turning at the limit
    settle();
    settle();

    let velocity_echoes = handle.messages("screen_string")[before..]
        .iter()
        .filter_map(|m| m.as_str())
        .filter(|s| s.starts_with("Dial:"))
        .count();
    assert_eq!(
        0, velocity_echoes,
        "no velocity echo at the accumulator limit"
    );
}

#[test]
fn echo_slider_suppresses_resting_value_during_settle_window() {
    // a slider reports its resting position (and analog settling) on connect; that
    // must not echo as a move. echoes within ECHO_SETTLE (1.5s) are suppressed;
    // a deliberate move after the window echoes.
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_echo(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let handle = ctrl.device().handle();
    let before = handle.messages("screen_string").len();
    ctrl.device().input_slide_up(7, 100); // within the settle window -> suppressed
    settle();
    settle();
    ctrl.device().input_slide_up(7, 150); // after the window -> a deliberate move
    settle();
    settle();

    let echoes: Vec<String> = handle.messages("screen_string")[before..]
        .iter()
        .filter_map(|m| m.as_str().map(str::to_string))
        .filter(|s| s.starts_with("Slider:"))
        .collect();
    // only the move echoes; the cumulative slide lands at 100 + 150 = 250.
    assert_eq!(
        1,
        echoes.len(),
        "only the move echoes, not the baseline: {echoes:?}"
    );
    assert!(
        echoes[0].contains("250"),
        "echo is the moved value: {}",
        echoes[0]
    );
}

#[test]
fn echo_orbiter_pad_shows_one_xy_coordinate() {
    // the orbiter pad reports both axes each frame; the echo collapses them into
    // a single (x, y) line centered at 0,0, so they do not flicker against each
    // other on the status display.
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_echo(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    // orbiter (index 2): pad at x=40, y=-40 (each a big-endian signed-16 axis).
    ctrl.device().input(2, [0, 0, 40, 255, 216, 0, 0, 0]);
    settle();

    assert_eq!("Orbiter: 40,-40", ctrl.screen_string_value());
}

#[test]
fn led_draw_paints_every_monogram_module() {
    // demoscene paints via led_draw across the slot grid. each module (core,
    // tri-dial, orbiter) must receive a color -- not just the multi-cell orbiter
    // whose cells come first in the grid walk and otherwise eat every pixel.
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    let (min_x, min_y, max_x, max_y) = ctrl.layout_extrema();
    let (w, h) = ((max_x - min_x).abs() + 1, (max_y - min_y).abs() + 1);
    let pixels = vec![(10, 20, 30); ctrl.connected_count()];

    let handle = ctrl.device().handle();
    let before = handle.messages("set_module").len();
    ctrl.led_draw(pixels, min_x, max_y, w, h);
    ctrl.wait();

    let msgs = handle.messages("set_module");
    let lit: std::collections::HashSet<i64> = msgs[before..]
        .iter()
        .filter_map(|m| m[0].as_i64())
        .collect();
    assert!(lit.contains(&0), "core lit");
    assert!(lit.contains(&1), "tri-dial lit");
    assert!(lit.contains(&2), "orbiter lit");
}

#[test]
fn monogram_layout_traverses_to_three_modules() {
    // the gui renders a built-in layout; the monogram one must traverse to the
    // core / tri-dial / orbiter at the device's 0-based indices and real slots.
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    assert_eq!(3, ctrl.connected_count());
    let kinds: Vec<(i64, &str)> = ctrl
        .render_components()
        .iter()
        .map(|c| (c.index, c.kind_name))
        .collect();
    assert_eq!(vec![(0, "Core"), (1, "Tri-Dial"), (2, "Orbiter")], kinds);
    assert_eq!(Some((0, 0)), ctrl.component_slot(0));
}

#[test]
fn inspect_summary_deduplicates_inputs_and_ranges() {
    let device = FakeDevice::new("DEAD:BEEF");
    device.components_connected(layouts::expertkit());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.set_inspect(true);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();

    // dial 2: two left rotations of different magnitude; button 6: press+release.
    ctrl.device().input_rotation_left(2, 8);
    ctrl.device().input_rotation_left(2, 16);
    ctrl.device().input_pressed(6);
    ctrl.device().input_released(6);
    settle();

    let (inputs, registers) = ctrl.inspect_summary();

    // the two rotations collapse to one row, with the value range spanning both.
    let rot = inputs
        .iter()
        .find(|r| r.index == 2 && r.label == "rotation_left")
        .unwrap();
    assert_eq!(2, rot.count);
    assert_eq!(Some((8, 16)), rot.value_range);

    let pressed = inputs
        .iter()
        .find(|r| r.index == 6 && r.label == "pressed")
        .unwrap();
    assert_eq!(1, pressed.count);
    assert_eq!(Some((1, 1)), pressed.value_range);

    // the rotation register range surfaces in the per-register view.
    let dial = registers.iter().find(|r| r.index == 2).unwrap();
    assert_eq!(Some((8, 16)), dial.registers[1]);
    assert_eq!(0, ctrl.failed_writes());
}

#[test]
fn app_screen_and_uid_led_choices_survive_refresh() {
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();
    assert!(ctrl.app_display_frame(vec![0; 12]).is_err());
    let uid = ctrl
        .render_components()
        .into_iter()
        .find(|m| m.kind == 9)
        .unwrap()
        .uid;
    ctrl.app_led_colors(vec![(uid, 1, (255, 0, 0))]);
    ctrl.wait();
    settle();
    let handle = ctrl.device().handle();
    assert!(handle
        .messages("set_module")
        .iter()
        .any(|m| *m == serde_json::json!([1, 5, 0x010000ff])));
    // Every pixel byte needs escaping: strip subdivision must still fit USB.
    let pixels: Vec<u8> = (0..115200)
        .map(|i| if i % 2 == 0 { 125 } else { 126 })
        .collect();
    ctrl.app_display_frame(pixels.clone()).unwrap();
    settle();
    ctrl.wait();
    settle();
    let writes = handle.messages("write_display_slow");
    assert!(writes.iter().any(|m| m[5]
        .as_array()
        .is_some_and(|p| p.first() == Some(&serde_json::json!(125)))));
    // Later screen text must retain the active app's frame.
    let before = writes.len();
    ctrl.screen_string("keep app image");
    settle();
    ctrl.wait();
    assert_eq!(before, handle.messages("write_display_slow").len());
    ctrl.app_display_frame(vec![0xf8; 115200]).unwrap();
    settle();
    ctrl.wait();
    settle();
    assert!(handle.messages("write_display_slow").len() > before);
    // A second application change must update even though internal title is APP.
    let second = handle.messages("write_display_slow").len();
    ctrl.app_display_frame(vec![0x07; 115200]).unwrap();
    settle();
    ctrl.wait();
    settle();
    assert!(handle.messages("write_display_slow").len() > second);
    ctrl.close();
}

#[test]
fn monogram_restores_normal_mode_and_pings_before_standby() {
    let mut device = FakeDevice::new("DEAD:BEEF");
    device.set_legacy(false);
    device.components_connected(layouts::monogram());
    let mut ctrl = Controller::new(Configuration::default(), device);
    ctrl.open().unwrap();
    ctrl.connect().unwrap();
    ctrl.wait_for_layout();
    settle();
    let handle = ctrl.device().handle();
    assert!(handle.messages("set_power_mode").contains(&serde_json::json!(0)));
    let before = handle.messages("get_check").len();
    sleep(Duration::from_millis(2600));
    assert!(handle.messages("get_check").len() > before);
    assert!(handle.messages("start").is_empty());
    ctrl.close();
}
