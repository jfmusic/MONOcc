//! the led demos: lavalamp (per-component random walk), lightshow (random
//! flashes), and demoscene (a scrolling sinebow with dial / button control).
//!
//! each runs until the user presses enter. demoscene polls dial and button
//! values, so its live control is exercised on real hardware (phase 6); under
//! `--simulator` there is no input source, so it shows the animation.

use std::f64::consts::PI;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rand::rngs::ThreadRng;
use rand::Rng;
use sextant_core::component::kind;
use sextant_core::controller::LED_DELAY;
use sextant_core::{Led, RenderComponent};

use crate::common::ControllerOps;

const COLOR_SHIFT: i64 = 4;

/// the per-frame led-write cadence, scaled by component count as upstream did.
fn led_delay(count: usize) -> Duration {
    LED_DELAY.mul_f64((count as f64 / 2.0).max(1.0))
}

/// run `body` each frame until the user presses enter (or stdin closes).
fn until_enter(mut body: impl FnMut()) {
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = std::io::stdin().read_line(&mut line);
        signal.store(true, Ordering::SeqCst);
    });
    eprintln!("press enter to stop.");
    while !stop.load(Ordering::SeqCst) {
        body();
    }
}

/// a per-component random walk in rgb, driven by the headless loop or the gui.
pub struct Lavalamp {
    leds: Vec<[i64; 3]>,
    indices: Vec<i64>,
    rng: ThreadRng,
}

impl Default for Lavalamp {
    fn default() -> Self {
        Self::new()
    }
}

impl Lavalamp {
    pub fn new() -> Lavalamp {
        Lavalamp {
            leds: Vec::new(),
            indices: Vec::new(),
            rng: rand::thread_rng(),
        }
    }

    /// advance one frame: nudge each component's color and write the batch. the
    /// connected set is snapshot on the first step.
    pub fn step(&mut self, controller: &dyn ControllerOps) {
        if self.indices.is_empty() {
            self.indices = controller.connected_indices();
            self.leds = vec![[220, 220, 220]; self.indices.len()];
        }
        for state in self.leds.iter_mut() {
            let channel = self.rng.gen_range(0..3);
            let change = if self.rng.gen_range(0..2) == 1 {
                -COLOR_SHIFT
            } else {
                COLOR_SHIFT
            };
            state[channel] = (state[channel] + change).clamp(0, 255);
        }
        let batch = self
            .indices
            .iter()
            .zip(&self.leds)
            .map(|(&i, s)| Led {
                i,
                r: s[0] as u8,
                g: s[1] as u8,
                b: s[2] as u8,
                m: 0,
            })
            .collect();
        controller.led(batch);
    }
}

/// the headless lavalamp: step on a fixed cadence until the user presses enter.
pub fn lavalamp(controller: &dyn ControllerOps) {
    let mut demo = Lavalamp::new();
    let delay = led_delay(controller.connected_count());
    until_enter(|| {
        demo.step(controller);
        std::thread::sleep(delay);
    });
}

/// random single-component flashes, driven by the headless loop or the gui.
pub struct Lightshow {
    rng: ThreadRng,
}

impl Default for Lightshow {
    fn default() -> Self {
        Self::new()
    }
}

impl Lightshow {
    pub fn new() -> Lightshow {
        Lightshow {
            rng: rand::thread_rng(),
        }
    }

    /// advance one frame: flash one random component a random color.
    pub fn step(&mut self, controller: &dyn ControllerOps) {
        let indices = controller.connected_indices();
        if indices.is_empty() {
            return;
        }
        let i = indices[self.rng.gen_range(0..indices.len())];
        controller.led(vec![Led {
            i,
            r: self.rng.gen(),
            g: self.rng.gen(),
            b: self.rng.gen(),
            m: 0,
        }]);
    }
}

/// the headless lightshow: step on a fixed cadence until the user presses enter.
pub fn lightshow(controller: &dyn ControllerOps) {
    let mut demo = Lightshow::new();
    until_enter(|| {
        demo.step(controller);
        std::thread::sleep(LED_DELAY);
    });
}

/// a sinebow preset: three sinusoids (one per channel) sampled over a length.
#[derive(Clone)]
struct Sinebow {
    title: &'static str,
    freq: [f64; 3],
    phase: [f64; 3],
    length: i64,
    center: f64,
    width: f64,
}

impl Sinebow {
    fn colors(&self) -> Vec<(u8, u8, u8)> {
        (0..self.length)
            .map(|i| {
                let i = i as f64;
                let channel = |c: usize| {
                    let v = (self.freq[c] * i + self.phase[c]).sin() * self.width + self.center;
                    (v as i64).clamp(0, 255) as u8
                };
                (channel(0), channel(1), channel(2))
            })
            .collect()
    }
}

fn default_presets() -> Vec<Sinebow> {
    vec![
        Sinebow {
            title: "USER",
            freq: [0.1, 0.1, 0.1],
            phase: [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0],
            length: 65,
            center: 128.0,
            width: 127.0,
        },
        Sinebow {
            title: "MOVE",
            freq: [1.0 / 3.0, 0.275, 0.486],
            phase: [0.455, 0.486, 1.286],
            length: 106,
            center: 63.0,
            width: 56.0,
        },
        Sinebow {
            title: "SLOW",
            freq: [0.1, 0.1, 0.1],
            phase: [0.0, 2.0 * PI / 3.0, 4.0 * PI / 3.0],
            length: 65,
            center: 128.0,
            width: 127.0,
        },
    ]
}

const CONTROLS: [&str; 3] = ["FREQ", "PHASE", "META"];
const CHANNELS: [char; 3] = ['R', 'G', 'B'];

fn rowwise_by_kind(controller: &dyn ControllerOps, component_kind: u8) -> Vec<RenderComponent> {
    let mut components: Vec<RenderComponent> = controller
        .render_components()
        .into_iter()
        .filter(|c| c.kind == component_kind)
        .collect();
    components.sort_by_key(|c| (-c.slot.1, c.slot.0));
    components
}

/// the press state of `count` buttons, device-agnostically. palettegear exposes
/// discrete Button modules; the monogram core reports its buttons as a bitmask
/// in v[0] (bit 1 = button a, bit 2 = button b), so read that when no discrete
/// buttons are present.
fn button_states(controller: &dyn ControllerOps, count: usize) -> Vec<bool> {
    let buttons = rowwise_by_kind(controller, kind::BUTTON);
    if !buttons.is_empty() {
        return (0..count)
            .map(|i| buttons.get(i).map(|b| b.values[0] == 1).unwrap_or(false))
            .collect();
    }
    match controller
        .render_components()
        .into_iter()
        .find(|c| c.kind == kind::CORE)
    {
        Some(core) => (0..count).map(|i| core.values[0] & (1 << i) != 0).collect(),
        None => vec![false; count],
    }
}

/// up to three dial positions (0-255), device-agnostically. palettegear dials
/// report an absolute set value in v[3]; the monogram tri-dial's three knobs are
/// delta-only, so the controller integrates them into accumulated[1..3].
fn dial_values(controller: &dyn ControllerOps) -> Vec<i64> {
    let dials = rowwise_by_kind(controller, kind::DIAL);
    if !dials.is_empty() {
        return dials.iter().take(3).map(|d| d.values[3]).collect();
    }
    match controller
        .render_components()
        .into_iter()
        .find(|c| c.kind == kind::TRI_DIAL)
    {
        Some(td) => vec![td.accumulated[1], td.accumulated[2], td.accumulated[3]],
        None => Vec::new(),
    }
}

/// apply a dial's set value to the user preset for the active control mode,
/// returning the screen label for the change.
fn apply_dial(preset: &mut Sinebow, control: usize, dial: usize, value: i64) -> String {
    match control {
        0 => {
            let freq = value as f64 / 255.0;
            preset.freq[dial] = freq;
            format!("HZ/{}: {freq:.6}", CHANNELS[dial])
        }
        1 => {
            let scaled = value as f64 * 4.0 / 255.0;
            preset.phase[dial] = scaled * PI / 3.0;
            format!("PH/{}: {scaled:.6}", CHANNELS[dial])
        }
        _ => {
            match dial {
                0 => preset.center = value as f64,
                1 => preset.width = value as f64,
                _ => preset.length = value.max(1),
            }
            format!("{}: {value}", ['C', 'W', 'L'][dial])
        }
    }
}

/// the live demoscene state, advanced one animation frame at a time so it can be
/// driven by either the headless loop or the simulator's render loop.
pub struct Demoscene {
    presets: Vec<Sinebow>,
    demo: usize,
    control: usize,
    colors: Vec<(u8, u8, u8)>,
    pixel_colors: Vec<usize>,
    prev_length: i64,
    last_dials: [i64; 3],
    prev_pressed: [bool; 2],
}

impl Default for Demoscene {
    fn default() -> Self {
        Self::new()
    }
}

impl Demoscene {
    pub fn new() -> Demoscene {
        let presets = default_presets();
        let colors = presets[0].colors();
        Demoscene {
            presets,
            demo: 0,
            control: 0,
            colors,
            pixel_colors: Vec::new(),
            prev_length: -1,
            last_dials: [i64::MIN; 3],
            prev_pressed: [false, false],
        }
    }

    /// advance one frame: poll dial/button input, scroll the palette, and paint.
    pub fn step(&mut self, controller: &dyn ControllerOps) {
        let count = controller.connected_count();

        // button releases cycle the demo (button 0) and control mode (button 1).
        let buttons = button_states(controller, self.prev_pressed.len());
        for (slot, &pressed) in buttons.iter().enumerate() {
            if self.prev_pressed[slot] && !pressed {
                if slot == 0 {
                    self.demo = (self.demo + 1) % self.presets.len();
                    self.colors = self.presets[self.demo].colors();
                    controller.screen_string(&format!("DEMO: {}", self.presets[self.demo].title));
                } else {
                    self.control = (self.control + 1) % CONTROLS.len();
                    controller.screen_string(&format!("MODE: {}", CONTROLS[self.control]));
                }
            }
            self.prev_pressed[slot] = pressed;
        }

        // dials drive the user preset's parameters for the active control mode.
        // the first read baselines (no change) so a device resting at 0 does not
        // zero the preset before the user touches anything.
        for (slot, value) in dial_values(controller).into_iter().take(3).enumerate() {
            if self.last_dials[slot] == i64::MIN {
                self.last_dials[slot] = value;
            } else if value != self.last_dials[slot] {
                self.last_dials[slot] = value;
                let label = apply_dial(&mut self.presets[0], self.control, slot, value);
                self.demo = 0;
                self.colors = self.presets[0].colors();
                controller.screen_string(&label);
            }
        }

        // scroll the palette across the component grid and paint it.
        if self.prev_length != self.presets[self.demo].length {
            self.pixel_colors = (0..count).collect();
            self.prev_length = self.presets[self.demo].length;
        }
        let palette_len = self.colors.len().max(1);
        for pixel in self.pixel_colors.iter_mut() {
            *pixel = (*pixel + 1) % palette_len;
        }
        if self.pixel_colors.is_empty() {
            return;
        }
        let pixels: Vec<(u8, u8, u8)> = (0..count)
            .map(|x| self.colors[self.pixel_colors[x % self.pixel_colors.len()]])
            .collect();
        let (min_x, min_y, max_x, max_y) = controller.layout_extrema();
        let w = (max_x - min_x).abs() + 1;
        let h = (max_y - min_y).abs() + 1;
        controller.led_draw(pixels, min_x, max_y, w, h);
    }
}

/// the headless demoscene: step on a fixed cadence until the user presses enter.
pub fn demoscene(controller: &dyn ControllerOps) {
    let mut demo = Demoscene::new();
    let delay = led_delay(controller.connected_count());
    until_enter(|| {
        demo.step(controller);
        std::thread::sleep(delay);
    });
}
