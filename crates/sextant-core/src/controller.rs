//! the threaded controller: it drives a device through its lifecycle and turns
//! component input into actions and led / screen output.
//!
//! the design mirrors the python original's worker model: a reader thread
//! decodes input and dispatches it, a writer thread drains a queue to the bulk
//! endpoint, rate-limited led and screen-string threads coalesce output, and a
//! keepalive thread re-pings and refreshes the screen. shared mutable state sits
//! behind a single mutex held only briefly, so the rate-limited workers are
//! never starved; the queues are crossbeam channels with the same bounded /
//! unbounded split and drop policy as upstream.

use crate::component::{
    kind, Behavior, BehaviorKind, CallbackOutcome, ComponentLayout, DispatchValue,
};
use crate::configuration::{action, icon, input, Configuration};
use crate::device::{self, Device, DeviceClass, DeviceHandle, UsbError};
use crate::inspect::{InspectAnalysis, InspectInputRow, InspectRegisterRow};
use crate::message::{self, MessageFormat};

use crossbeam_channel::{
    bounded, unbounded, Receiver, RecvTimeoutError, SendTimeoutError, Sender, TrySendError,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const QUEUE_TIMEOUT: Duration = Duration::from_millis(500);
pub const DEFAULT_WAIT: Duration = Duration::from_millis(500);
pub const SCREEN_STRING_DELAY: Duration = Duration::from_millis(50); // 1/20s
pub const LED_DELAY: Duration = Duration::from_micros(16_666); // 1/60s
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(5);
// The original CoreV2 service pings every two seconds, including while idle.
pub const MONOGRAM_KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(2);
pub const SCREEN_RESET: Duration = Duration::from_secs(3);
pub const BULK_READ_TIMEOUT: Duration = Duration::from_secs(1);
// the largest single bulk-out transfer the palette core absorbs without a
// watchdog reset (measured with `probe`: survives 2048B, resets at 3072B). a
// connect must never send a transfer this large -- the ~8kb icon upload is the
// explicit `upload` command, not part of connecting.
pub const MAX_SAFE_TRANSFER: usize = 2048;
// after connect, suppress slider echoes this long while resting values settle.
pub const ECHO_SETTLE: Duration = Duration::from_millis(1500);
const SCREEN_QUEUE_DEPTH: usize = 8;
const LED_QUEUE_DEPTH: usize = 64;
const FIRMWARE_VERSION_SLIDER_FIXED: &str = "1.3.2";
// the monogram core screen is 240x240 rgb565, uploaded in horizontal strips.
const SCREEN_DIM: usize = 240;
const SCREEN_STRIP_ROWS: usize = 4;
// host-rendered status text: 8x8 glyphs scaled, drawn near the bottom in white.
const SCREEN_TEXT_SCALE: usize = 2;
const SCREEN_TEXT_TOP: usize = 210;
const SCREEN_TEXT_COLOR: u16 = 0xFFFF;

/// a component index paired with the hid bindings to program onto it.
type ComponentHidkeys = (i64, Vec<crate::component::HidKey>);

/// a single component led command.
#[derive(Clone, Copy, Debug)]
pub struct Led {
    pub i: i64,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub m: u8,
}

impl Led {
    fn to_json(self) -> Value {
        json!({"b": self.b, "g": self.g, "i": self.i, "m": self.m, "r": self.r})
    }
}

/// a snapshot of one component's renderable state, handed to the simulator.
#[derive(Clone, Debug)]
pub struct RenderComponent {
    pub index: i64,
    pub uid: String,
    pub kind: u8,
    pub kind_name: &'static str,
    pub slot: (i32, i32),
    pub orientation: i32,
    pub led: (u8, u8, u8),
    pub screen_string: String,
    pub screen_display: u8,
    pub values: [i64; 8],
    pub accumulated: [i64; 8],
    pub footprint: (i32, i32),
    pub behavior: Option<&'static str>,
}

/// mutable controller state guarded by a single, briefly-held mutex.
struct Inner {
    config: Configuration,
    layout: ComponentLayout,
    current_led_values: (i64, i64, i64),
    current_screen_string_value: String,
    version_core: String,
    layout_time: Option<Value>,
    last_write_time: Instant,
    last_led_time: Instant,
    last_screen_string_time: Instant,
    failed_writes: usize,
    failed_reads: usize,
    dropped_leds: usize,
    dropped_screen_strings: usize,
    // the last 240x240 rgb565 frame uploaded to the monogram core screen, so a
    // redraw re-sends only the strips that changed. `None` forces a full upload
    // (first draw, or after a failed strip leaves the device's buffer unknown).
    last_display_frame: Option<Vec<u8>>,
    app_display_frame: Option<Vec<u8>>,
    app_led_colors: std::collections::HashMap<(String, usize), (u8, u8, u8)>,
    // until this instant, slider echoes are suppressed: a connecting device
    // reports each slider's resting position (and analog settling over several
    // frames), which should not echo as if the user just moved it.
    echo_live_at: Instant,
}

/// per-frame hints for the on-screen input echo, derived while the input is
/// decoded under lock and consumed when echoing.
struct EchoHints {
    // accumulator controls (dial / tri-dial / orbiter) echo their position, not
    // the rotation velocity, even at the 0/255 limits.
    has_accumulator: bool,
    // a slider's resting value reported on connect is not echoed as a move.
    skip_baseline: bool,
    // the orbiter pad as one (x, y) coordinate centered at 0,0, so the two axes
    // do not fight over the single status line.
    joystick: Option<(i64, i64)>,
}

/// everything the worker threads share. methods take `&self` (an `Arc<Shared>`).
struct Shared {
    handle: Arc<dyn DeviceHandle>,
    class: DeviceClass,
    format: MessageFormat,
    // connect instant, so debug traces can be read as elapsed milliseconds.
    started_at: Instant,
    map_midi: bool,
    echo: bool,
    debug: bool,
    inspect: bool,
    inspect_acc: Mutex<InspectAnalysis>,
    inner: Mutex<Inner>,
    write_tx: Sender<Vec<u8>>,
    write_rx: Receiver<Vec<u8>>,
    led_tx: Sender<Vec<Led>>,
    led_rx: Receiver<Vec<Led>>,
    screen_tx: Sender<String>,
    screen_rx: Receiver<String>,
    shutdown: AtomicBool,
}

/// the controller, generic over the device backend (real or fake).
pub struct Controller<D: Device> {
    device: D,
    config: Option<Configuration>,
    shared: Option<Arc<Shared>>,
    threads: Vec<JoinHandle<()>>,
    class: DeviceClass,
    map_midi: bool,
    echo: bool,
    debug: bool,
    inspect: bool,
}

impl<D: Device> Controller<D> {
    pub fn new(config: Configuration, device: D) -> Controller<D> {
        let class = device.class();
        Controller {
            device,
            config: Some(config),
            shared: None,
            threads: Vec::new(),
            class,
            map_midi: true,
            echo: false,
            debug: false,
            inspect: false,
        }
    }

    /// in echo mode, every input event shows its value on the base screen.
    pub fn set_echo(&mut self, echo: bool) {
        self.echo = echo;
    }

    /// in debug mode, every low-level message sent to or received from the device
    /// is printed to stderr -- useful for learning an unrecognized component.
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// in inspect mode, every input event prints a decoded one-line summary
    /// (raw registers, what changed, signed reading, and the derived input
    /// types) to stdout, to learn or extend a component's input format.
    pub fn set_inspect(&mut self, inspect: bool) {
        self.inspect = inspect;
    }

    /// the deduplicated input summary accumulated while inspecting: per-control
    /// rows (counts + value ranges) and the per-register raw activity.
    pub fn inspect_summary(&self) -> (Vec<InspectInputRow>, Vec<InspectRegisterRow>) {
        let acc = self.shared().inspect_acc.lock().unwrap();
        (acc.input_rows(), acc.register_rows())
    }

    /// the raw layout tree the device last reported, so the gui can render the
    /// actually-attached device rather than a built-in layout.
    pub fn layout_tree(&self) -> Value {
        self.shared()
            .inner
            .lock()
            .unwrap()
            .layout
            .layout_dict
            .clone()
    }

    /// the layout's base orientation in degrees, for the gui to rotate the core
    /// screen with the rest of the setup.
    pub fn layout_orientation(&self) -> i32 {
        self.shared().inner.lock().unwrap().layout.base_orientation
    }

    /// the most recent base-screen status string (for tests / introspection).
    pub fn screen_string_value(&self) -> String {
        self.shared()
            .inner
            .lock()
            .unwrap()
            .current_screen_string_value
            .clone()
    }

    /// the underlying device, for tests and input injection.
    pub fn device(&self) -> &D {
        &self.device
    }

    fn shared(&self) -> &Arc<Shared> {
        self.shared.as_ref().expect("controller is not open")
    }

    /// claim the device interface and build the shared worker state.
    pub fn open(&mut self) -> Result<(), UsbError> {
        let handle = self.device.open()?;
        let config = self.config.take().expect("config already consumed");
        let layout = ComponentLayout::new(config.profile.orientation);
        let format = if self.class == DeviceClass::PaletteGear {
            MessageFormat::Json
        } else {
            MessageFormat::Msgpack
        };
        let now = Instant::now();
        // start the screen clock far in the past so the first keepalive tick
        // performs a screen reset (upstream initializes these timers to the
        // epoch); the write/led clocks start now since the connect sequence
        // writes immediately.
        let stale = now.checked_sub(Duration::from_secs(3600)).unwrap_or(now);
        let (write_tx, write_rx) = unbounded();
        let (led_tx, led_rx) = bounded(LED_QUEUE_DEPTH);
        let (screen_tx, screen_rx) = bounded(SCREEN_QUEUE_DEPTH);
        self.shared = Some(Arc::new(Shared {
            handle,
            class: self.class,
            format,
            started_at: now,
            map_midi: self.map_midi,
            echo: self.echo,
            debug: self.debug,
            inspect: self.inspect,
            inspect_acc: Mutex::new(InspectAnalysis::new()),
            inner: Mutex::new(Inner {
                config,
                layout,
                current_led_values: (255, 255, 255),
                current_screen_string_value: String::new(),
                version_core: String::new(),
                layout_time: None,
                last_write_time: now,
                last_led_time: now,
                last_screen_string_time: stale,
                failed_writes: 0,
                failed_reads: 0,
                dropped_leds: 0,
                dropped_screen_strings: 0,
                last_display_frame: None,
                app_display_frame: None,
                app_led_colors: Default::default(),
                echo_live_at: now,
            }),
            write_tx,
            write_rx,
            led_tx,
            led_rx,
            screen_tx,
            screen_rx,
            shutdown: AtomicBool::new(false),
        }));
        Ok(())
    }

    /// run the connection sequence: drain, handshake, spawn workers, initialize.
    pub fn connect(&mut self) -> Result<(), UsbError> {
        let shared = self.shared().clone();
        shared.bulk_read();
        shared.control_handshake()?;
        self.spawn(&shared);
        shared.initialize();
        let keepalive = shared.clone();
        self.threads
            .push(thread::spawn(move || keepalive.keepalive_worker()));
        Ok(())
    }

    fn spawn(&mut self, shared: &Arc<Shared>) {
        for worker in [
            Shared::bulk_read_worker as fn(&Shared),
            Shared::bulk_write_worker,
            Shared::screen_string_worker,
            Shared::led_worker,
        ] {
            let s = shared.clone();
            self.threads.push(thread::spawn(move || worker(&s)));
        }
    }

    pub fn wait_for_layout(&self) {
        self.shared().wait_for_layout();
    }

    /// block until the output queues have drained.
    pub fn wait(&self) {
        self.shared().wait();
    }

    pub fn print_stats(&self) {
        let inner = self.shared().inner.lock().unwrap();
        println!("[@] cumulative dropped_leds: {}", inner.dropped_leds);
        println!(
            "[@] cumulative dropped_screen_strings: {}",
            inner.dropped_screen_strings
        );
        println!("[@] cumulative failed_writes: {}", inner.failed_writes);
    }

    /// signal shutdown, join the workers, and release the interface.
    pub fn close(&mut self) {
        if let Some(shared) = &self.shared {
            shared.shutdown.store(true, Ordering::SeqCst);
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        if let Some(shared) = &self.shared {
            shared.handle.reset();
            shared.handle.release();
        }
    }

    // -- accessors used by integration tests and the cli --

    pub fn is_shutdown(&self) -> bool {
        self.shared().shutdown.load(Ordering::SeqCst)
    }

    pub fn connected_count(&self) -> usize {
        self.shared().inner.lock().unwrap().layout.len()
    }

    /// indices of the connected components, in attachment order.
    pub fn connected_indices(&self) -> Vec<i64> {
        let inner = self.shared().inner.lock().unwrap();
        inner.layout.connected().iter().map(|c| c.index).collect()
    }

    /// rotate the whole layout by `rotation` degrees and reprocess it, used by
    /// the simulator's tab key.
    pub fn rotate(&self, rotation: i32) {
        self.shared().rotate(rotation);
    }

    /// the slot bounding box of the connected layout (min_x, min_y, max_x, max_y).
    pub fn layout_extrema(&self) -> (i32, i32, i32, i32) {
        self.shared().inner.lock().unwrap().layout.extrema()
    }

    /// a snapshot of each connected component's renderable state, for the gui.
    pub fn render_components(&self) -> Vec<RenderComponent> {
        let inner = self.shared().inner.lock().unwrap();
        inner
            .layout
            .connected()
            .iter()
            .map(|c| RenderComponent {
                index: c.index,
                uid: c.uid.clone(),
                kind: c.kind,
                kind_name: c.kind_name(),
                slot: c.slot.unwrap_or((0, 0)),
                orientation: c.orientation.unwrap_or(0),
                led: c.led,
                screen_string: c.screen_string.clone(),
                screen_display: c.screen_display,
                values: c.values,
                accumulated: c.accumulated,
                footprint: c.footprint,
                behavior: c.behavior_label(),
            })
            .collect()
    }

    pub fn screen_string(&self, title: &str) {
        self.shared().screen_string(title.to_string());
    }

    /// Supply the app profile's rendered 240x240 big-endian RGB565 screen.
    pub fn app_display_frame(&self, pixels: Vec<u8>) -> Result<(), String> {
        if pixels.len() != SCREEN_DIM * SCREEN_DIM * 2 {
            return Err("screen frame must contain exactly 115200 bytes".into());
        }
        {
            let mut inner = self.shared().inner.lock().unwrap();
            inner.app_display_frame = Some(pixels);
            inner.current_screen_string_value.clear();
        }
        self.shared().screen_string("APP".to_string());
        Ok(())
    }

    /// Replace LED choices for the active application, addressed by stable UID.
    pub fn app_led_colors(&self, colors: Vec<(String, usize, (u8, u8, u8))>) {
        self.shared().inner.lock().unwrap().app_led_colors = colors
            .into_iter()
            .map(|(uid, sub, rgb)| ((uid, sub), rgb))
            .collect();
        self.shared().configure_leds(None, None, None);
    }

    pub fn component_values(&self, index: i64) -> Option<[i64; 8]> {
        let inner = self.shared().inner.lock().unwrap();
        inner.layout.find_component(index).map(|c| c.values)
    }

    pub fn component_orientation(&self, index: i64) -> Option<i32> {
        let inner = self.shared().inner.lock().unwrap();
        inner
            .layout
            .find_component(index)
            .and_then(|c| c.orientation)
    }

    pub fn component_slot(&self, index: i64) -> Option<(i32, i32)> {
        let inner = self.shared().inner.lock().unwrap();
        inner.layout.find_component(index).and_then(|c| c.slot)
    }

    pub fn component_flip(&self, index: i64) -> Option<bool> {
        let inner = self.shared().inner.lock().unwrap();
        inner.layout.find_component(index).map(|c| c.flip)
    }

    pub fn failed_writes(&self) -> usize {
        self.shared().inner.lock().unwrap().failed_writes
    }

    pub fn dropped_leds(&self) -> usize {
        self.shared().inner.lock().unwrap().dropped_leds
    }

    pub fn dropped_screen_strings(&self) -> usize {
        self.shared().inner.lock().unwrap().dropped_screen_strings
    }

    /// upload the embedded reference screen images while only the base is
    /// connected, matching upstream `configure_images`.
    pub fn configure_images(&self) {
        self.shared().configure_images();
    }

    /// upload explicit `(slot, raw)` base-screen images (e.g. `resource upload`).
    pub fn upload_images(&self, images: &[(i64, Vec<u8>)]) {
        self.shared().upload_screen_images(images);
    }

    /// enqueue a batch of led commands (used by the demos).
    pub fn led(&self, leds: Vec<Led>) {
        self.shared().led(leds);
    }

    /// paint a row-major pixel buffer onto the components in a slot window.
    pub fn led_draw(&self, pixels: Vec<(u8, u8, u8)>, x: i32, y: i32, w: i32, h: i32) {
        self.shared().led_draw(pixels, x, y, w, h);
    }

    /// set every component's led to the given color, leaving `None` channels.
    pub fn configure_leds(&self, red: Option<i64>, green: Option<i64>, blue: Option<i64>) {
        self.shared().configure_leds(red, green, blue);
    }

    pub fn screen_display(&self, index: i64) {
        self.shared().screen_display(index);
    }

    pub fn check(&self) {
        self.shared().check();
    }

    /// reboot the device into its dfu bootloader (only with the base attached).
    pub fn boot(&self) {
        self.shared().boot();
    }

    pub fn configure_hidmap(&self) {
        self.shared().configure_hidmap();
    }

    pub fn configure_midimap(&self) {
        self.shared().configure_midimap();
    }
}

impl<D: Device> Drop for Controller<D> {
    fn drop(&mut self) {
        self.close();
    }
}

impl Shared {
    fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// whether the device speaks the legacy palettegear json protocol; the
    /// device class is the source of truth, this is the convenience for the
    /// many protocol forks that only need the two-way distinction.
    fn legacy(&self) -> bool {
        self.class == DeviceClass::PaletteGear
    }

    // -- message helpers --

    fn bulk_write_messages(&self, messages: Vec<Value>) {
        if messages.is_empty() {
            return;
        }
        if self.debug {
            let ms = self.started_at.elapsed().as_millis();
            for message in &messages {
                eprintln!("[{ms:>6}] [send] {message}");
            }
        }
        let data = message::encode(&messages, self.format, &[]);
        let _ = self.write_tx.send(data);
    }

    /// read until a clean message boundary, returning the decoded messages.
    fn bulk_read(&self) -> Vec<Value> {
        let mut complete = Vec::new();
        let mut extra: Vec<u8> = Vec::new();
        loop {
            match self.handle.bulk_read(
                device::ENDPOINT_BULK_IN,
                device::BULK_READ_SIZE_BYTES,
                BULK_READ_TIMEOUT,
            ) {
                Ok(data) => {
                    extra.extend_from_slice(&data);
                    let (messages, remainder) = message::decode(&extra, self.format);
                    complete.extend(messages);
                    extra = remainder;
                    if extra.is_empty() {
                        return complete;
                    }
                }
                Err(err) => {
                    self.inner.lock().unwrap().failed_reads += 1;
                    self.process_usb_error(&err);
                    return Vec::new();
                }
            }
        }
    }

    fn process_usb_error(&self, err: &UsbError) {
        if matches!(err, UsbError::Disconnected) {
            self.shutdown.store(true, Ordering::SeqCst);
        }
    }

    // -- lifecycle --

    fn control_handshake(&self) -> Result<(), UsbError> {
        device::control_handshake(self.handle.as_ref())
    }

    fn initialize(&self) {
        self.start();
        self.clear();
        self.send_version();
        self.wait_for_layout();
        self.check();
    }

    fn start(&self) {
        if self.legacy() {
            self.bulk_write_messages(vec![json!({"start": 1})]);
        } else {
            // Restore host-controlled operation after the vendor app leaves
            // CoreV2 in no_app_standby (set_power_mode: 1) on exit.
            self.bulk_write_messages(vec![json!({"set_power_mode": 0})]);
        }
    }

    fn clear(&self) {
        self.bulk_write_messages(vec![
            json!({"hidmap": []}),
            json!({"joymap": []}),
            json!({"midimap": []}),
            json!({"led": []}),
        ]);
    }

    fn check(&self) {
        let message = if self.legacy() {
            json!({"check": 1})
        } else {
            json!({"get_check": true})
        };
        self.bulk_write_messages(vec![message]);
    }

    fn send_version(&self) {
        let message = if self.legacy() {
            json!({"send_version": 1})
        } else {
            json!({"get_version": true})
        };
        self.bulk_write_messages(vec![message]);
    }

    fn wait_for_layout(&self) {
        if !self.legacy() {
            self.bulk_write_messages(vec![json!({"get_layout": true})]);
        }
        loop {
            if !self.inner.lock().unwrap().layout.is_empty() {
                return;
            }
            thread::sleep(DEFAULT_WAIT);
        }
    }

    fn wait(&self) {
        while !self.led_rx.is_empty() || !self.screen_rx.is_empty() || !self.write_rx.is_empty() {
            thread::sleep(DEFAULT_WAIT);
        }
    }

    // -- output: leds --

    fn led(&self, leds: Vec<Led>) {
        if let Err(SendTimeoutError::Timeout(_)) = self.led_tx.send_timeout(leds, QUEUE_TIMEOUT) {
            self.inner.lock().unwrap().dropped_leds += 1;
        }
    }

    fn configure_leds(&self, red: Option<i64>, green: Option<i64>, blue: Option<i64>) {
        let leds = {
            let mut inner = self.inner.lock().unwrap();
            let (cr, cg, cb) = inner.current_led_values;
            let (r, g, b) = (red.unwrap_or(cr), green.unwrap_or(cg), blue.unwrap_or(cb));
            inner.current_led_values = (r, g, b);
            inner
                .layout
                .connected()
                .iter()
                .map(|c| Led {
                    i: c.index,
                    r: r as u8,
                    g: g as u8,
                    b: b as u8,
                    m: 0,
                })
                .collect::<Vec<_>>()
        };
        self.led(leds);
    }

    /// flash each component off, white, then the current color (a power-on test).
    fn cycle(&self) {
        let (indices, current) = {
            let inner = self.inner.lock().unwrap();
            let indices: Vec<i64> = inner.layout.connected().iter().map(|c| c.index).collect();
            (indices, inner.current_led_values)
        };
        let (cr, cg, cb) = current;
        for i in indices {
            self.led(vec![Led {
                i,
                r: 255,
                g: 0,
                b: 0,
                m: 0,
            }]);
            self.led(vec![Led {
                i,
                r: 255,
                g: 255,
                b: 255,
                m: 0,
            }]);
            self.led(vec![Led {
                i,
                r: cr as u8,
                g: cg as u8,
                b: cb as u8,
                m: 0,
            }]);
        }
    }

    // -- output: screen --

    fn screen_string(&self, title: String) {
        {
            let mut inner = self.inner.lock().unwrap();
            if title == inner.current_screen_string_value {
                // the same value re-asserted (e.g. a dial held against the 0/255
                // limit): keep it on screen instead of letting the keepalive
                // revert to the title, but do not re-render it.
                inner.last_screen_string_time = Instant::now();
                return;
            }
        }
        // never block the input/read loop on the screen queue -- a fast input
        // (the orbiter pad) would otherwise stall the read loop and back the
        // device's events up. but a status line only needs the *latest* reading,
        // so when the queue is full drop the oldest pending echo to make room,
        // never the newest (which would lose the value the user settled on).
        let mut title = title;
        loop {
            match self.screen_tx.try_send(title) {
                Ok(()) => break,
                Err(TrySendError::Full(pending)) => {
                    title = pending;
                    if self.screen_rx.try_recv().is_err() {
                        break;
                    }
                    self.inner.lock().unwrap().dropped_screen_strings += 1;
                }
                Err(TrySendError::Disconnected(_)) => break,
            }
        }
    }

    fn screen_display(&self, index: i64) {
        let message = if self.legacy() {
            json!({"screen_display": index})
        } else {
            json!({"invoke_display": index})
        };
        self.bulk_write_messages(vec![message]);
        let mut inner = self.inner.lock().unwrap();
        if let Some(component) = inner.layout.base_mut() {
            component.screen_display = index as u8;
        }
    }

    fn screen_write(&self, image: &[u8], index: i64) {
        if self.debug {
            eprintln!(
                "[{:>6}] [screen] upload image -> slot {index} ({} bytes)",
                self.started_at.elapsed().as_millis(),
                image.len()
            );
        }
        let data = message::encode(&[json!({"screen_write": index})], self.format, image);
        self.bulk_transfer(&data);
    }

    // -- layout configuration --

    fn configure_orientation(&self) {
        let messages = {
            let inner = self.inner.lock().unwrap();
            inner
                .layout
                .connected()
                .iter()
                .map(|c| json!({"flip": [{"i": c.index, "m": if c.flip { 1 } else { 0 }}]}))
                .collect()
        };
        self.bulk_write_messages(messages);
    }

    fn configure_display(&self) {
        let (orientation, icon, title) = {
            let mut inner = self.inner.lock().unwrap();
            let orientation = (180 + inner.config.profile.orientation).rem_euclid(360) / 90;
            let icon = inner.config.profile.icon;
            let title = inner.config.profile.title.clone();
            if let Some(component) = inner.layout.base_mut() {
                component.screen_string = title.clone();
                component.screen_display = icon;
            }
            (orientation, icon, title)
        };
        if self.legacy() {
            // select the icon slot and render text. the bitmap is NOT uploaded on
            // connect: a full-screen (~8kb) screen_write resets the palette core
            // (its receive buffer is ~2kb), so image uploads happen only via the
            // explicit `push-assets` command, never on every layout.
            self.bulk_write_messages(vec![
                json!({"screen_orientation": orientation}),
                json!({"screen_display": icon}),
                json!({"screen_string": title}),
            ]);
        } else {
            self.configure_screen();
        }
    }

    /// render the monogram core screen, which has no device-side text / icon
    /// rendering: the host rasterizes `text` over a 240x240 rgb565 frame and
    /// uploads it. the background is plain black (the brand-icon slot no longer
    /// composites a distinct logo). the device physically turns with the setup, so
    /// the frame is counter-rotated (-orientation) to keep the content upright.
    fn render_screen(&self, text: &str) {
        let app_frame = self.inner.lock().unwrap().app_display_frame.clone();
        if let Some(frame) = app_frame {
            self.write_display(&frame);
            return;
        }
        let (orientation, logo) = {
            let inner = self.inner.lock().unwrap();
            (
                inner.layout.base_orientation,
                inner.config.profile.icon == icon::PALETTE,
            )
        };
        let counter = (360 - orientation).rem_euclid(360);
        // the logo / font are read from the configuration home; absent (before
        // `sextant init`) the frame is a plain black background with no text.
        let logo_bytes = logo.then(crate::resources::monogram_logo).flatten();
        let font = crate::resources::monogram_font();
        let frame = screen_frame(text, logo_bytes.as_deref(), font.as_deref());
        self.write_display(&rotate_frame(&frame, counter));
    }

    /// draw the monogram core screen for the current profile (logo + title).
    fn configure_screen(&self) {
        let title = self.inner.lock().unwrap().config.profile.title.clone();
        self.render_screen(&title);
    }

    /// upload a 240x240 rgb565 frame to the core display in 4-row strips, then
    /// commit it. see docs/monogram-protocol.md. big-endian rgb565 is confirmed:
    /// a captured frame reconstructs to a clean image only when decoded that way.
    ///
    /// only the strips that differ from the last uploaded frame are re-sent: a
    /// status-text update over the static logo touches a handful of strips
    /// instead of all 60, and an unchanged re-render (the keepalive) sends
    /// nothing. the device retains un-rewritten strips in its buffer, so the
    /// commit still refreshes a complete frame.
    fn write_display(&self, pixels: &[u8]) {
        let strips = {
            let inner = self.inner.lock().unwrap();
            changed_strips(
                inner.last_display_frame.as_deref(),
                pixels,
                SCREEN_DIM,
                SCREEN_STRIP_ROWS,
            )
        };
        if strips.is_empty() {
            return;
        }
        let failed_before = self.inner.lock().unwrap().failed_writes;
        let stride = SCREEN_DIM * 2;
        for y in strips {
            let mut row = y;
            while row < y + SCREEN_STRIP_ROWS {
                let mut rows = y + SCREEN_STRIP_ROWS - row;
                let data = loop {
                    let strip = &pixels[row * stride..(row + rows) * stride];
                    let data = message::encode_display_strip(
                        true,
                        0,
                        row as i64,
                        SCREEN_DIM as i64,
                        (row + rows) as i64,
                        strip,
                    );
                    if data.len() <= MAX_SAFE_TRANSFER || rows == 1 {
                        break data;
                    }
                    rows = (rows / 2).max(1);
                };
                self.bulk_transfer(&data);
                row += rows;
            }
        }
        let commit = message::encode(
            &[json!({"invoke_display": 3}), json!({"invoke_display": 4})],
            self.format,
            &[],
        );
        self.bulk_transfer(&commit);
        // trust the cache only if every strip landed; a failure leaves the
        // device's buffer unknown, so force a full re-upload next time.
        let mut inner = self.inner.lock().unwrap();
        inner.last_display_frame = (inner.failed_writes == failed_before).then(|| pixels.to_vec());
    }

    fn configure_component_callbacks(&self) {
        let mut inner = self.inner.lock().unwrap();
        let indices: Vec<i64> = inner.layout.connected().iter().map(|c| c.index).collect();
        for index in &indices {
            if let Some(component) = inner.layout.find_component_mut(*index) {
                component.reset();
            }
        }
        let mut bindings: Vec<(String, i64)> = Vec::new();
        for query in inner.config.components() {
            if let Some(component) = inner.layout.query(&query).first() {
                bindings.push((query, component.index));
            }
        }
        for (query, index) in bindings {
            self.set_component_callbacks(&mut inner, &query, index);
        }
    }

    /// bind every configured action for one component (matched by `uid_query`).
    fn set_component_callbacks(&self, inner: &mut Inner, uid_query: &str, index: i64) {
        let Some(kind) = inner.layout.find_component(index).map(|c| c.kind) else {
            return;
        };
        let profiles = inner.config.profiles.len();
        for (input_type, action_name, action_config) in inner.config.component_actions(uid_query) {
            if !is_known_action(&action_name) {
                if self.debug {
                    eprintln!("[callback] skip {uid_query} {input_type} -> {action_name} (unknown action)");
                }
                continue;
            }
            let mut behavior =
                make_action_behavior(&action_config, kind, &input_type, &action_name);
            let mut data = Vec::new();
            match action_name.as_str() {
                action::EXECUTE_COMMAND => {
                    data.push(action_config.get("argv").cloned().unwrap_or(Value::Null))
                }
                action::CYCLE_MAPPING => {
                    data.push(action_config.get("target").cloned().unwrap_or(Value::Null))
                }
                action::PROFILE_SWITCH => behavior.set_item_count(profiles),
                _ => {}
            }
            if let Some(component) = inner.layout.find_component_mut(index) {
                component.set_callback(&input_type, &action_name, behavior, data);
            }
            if self.debug {
                eprintln!("[callback] bind i={index} {uid_query} {input_type} -> {action_name}");
            }
        }
    }

    fn configure_component_hidkeys(&self) {
        let mut inner = self.inner.lock().unwrap();
        let bindings: Vec<ComponentHidkeys> = inner
            .layout
            .connected()
            .iter()
            .map(|c| (c.index, inner.config.component_hidkeys(&c.uid)))
            .collect();
        for (index, hidkeys) in bindings {
            if let Some(component) = inner.layout.find_component_mut(index) {
                for (control, key, modifier, repeat) in hidkeys {
                    component.set_hidkey(&control, key, modifier, repeat);
                }
            }
        }
    }

    fn configure_midimap(&self) {
        let components: Vec<(i64, u8, String)> = {
            let inner = self.inner.lock().unwrap();
            inner
                .layout
                .connected()
                .iter()
                .map(|c| (c.index, c.kind, c.uid.clone()))
                .collect()
        };
        for (index, component_kind, uid) in components {
            let note = component_hash(&uid);
            let message = match component_kind {
                kind::DIAL => Some(json!({"midimap": [{"i": index, "s": [
                    {"c": 3, "d": note, "m": 1, "mc": 0, "t": 2},
                    {"c": 0, "d": note, "m": 0, "mc": 0, "t": 1}
                ]}]})),
                kind::BUTTON => Some(json!({"midimap": [{"i": index, "s": [
                    {"c": 0, "d": note, "m": 0, "mc": 0, "t": 1}
                ]}]})),
                kind::SLIDER => Some(json!({"midimap": [{"i": index, "s": [
                    {"c": 0, "d": note, "m": 0, "mc": 0, "t": 2}
                ]}]})),
                _ => None,
            };
            if let Some(message) = message {
                self.bulk_write_messages(vec![message]);
            }
        }
    }

    /// program the bundled base-screen images: slot 23 (the black base) plus the
    /// brand icon 0 when its (gitignored) reference asset is present, else skipped.
    fn configure_images(&self) {
        let images: Vec<(i64, Vec<u8>)> = [0u32, 23]
            .into_iter()
            .filter_map(|i| {
                crate::resources::asset_bytes(i, "raw").map(|b| (i as i64, b.into_owned()))
            })
            .collect();
        self.upload_screen_images(&images);
    }

    /// upload the given `(slot, raw)` base-screen images onto the device in order.
    /// this does not `clear()` -- an image upload should leave the led / hid / midi
    /// maps (and the leds) alone rather than resetting them.
    fn upload_screen_images(&self, images: &[(i64, Vec<u8>)]) {
        self.check();
        for (slot, image) in images {
            eprintln!("uploading slot {slot} ({} bytes)", image.len());
            self.screen_write(image, *slot);
            self.screen_display(*slot);
            thread::sleep(Duration::from_millis(50));
        }
        self.bulk_write_messages(vec![json!({"set_version_screen": 0})]);
        self.start();
    }

    /// map a row-major pixel buffer (top row first) onto components by slot.
    fn led_draw(&self, pixels: Vec<(u8, u8, u8)>, x: i32, y: i32, w: i32, h: i32) {
        let leds = {
            let inner = self.inner.lock().unwrap();
            let mut pixels = pixels.into_iter();
            let mut leds = Vec::new();
            // one pixel per component, not per cell: a multi-cell module (the
            // monogram orbiter / core / tri-dial) occupies several grid cells but
            // must consume a single pixel, else its cells eat the whole buffer and
            // later modules go dark.
            let mut seen = std::collections::HashSet::new();
            let mut cy = y;
            while cy > y - h {
                for cx in x..x + w {
                    if let Some(component) = inner.layout.component_in_slot((cx, cy)) {
                        if seen.insert(component.index) {
                            if let Some((r, g, b)) = pixels.next() {
                                leds.push(Led {
                                    i: component.index,
                                    r,
                                    g,
                                    b,
                                    m: 0,
                                });
                            }
                        }
                    }
                }
                cy -= 1;
            }
            leds
        };
        self.led(leds);
    }

    fn boot(&self) {
        if self.inner.lock().unwrap().layout.len() > 1 {
            return;
        }
        self.check();
        self.screen_display(icon::BLANK as i64);
        self.screen_string("Bootloader".to_string());
        thread::sleep(Duration::from_secs(1));
        self.bulk_write_messages(vec![json!({"boot": 1})]);
        self.wait();
        thread::sleep(Duration::from_secs(1));
    }

    fn configure_hidmap(&self) {
        let bindings: Vec<ComponentHidkeys> = {
            let inner = self.inner.lock().unwrap();
            inner
                .layout
                .connected()
                .iter()
                .map(|c| (c.index, c.hidkeys.clone()))
                .collect()
        };
        for (index, hidkeys) in bindings {
            for (control, key, modifier, repeat) in hidkeys {
                let entry = json!({
                    "i": index,
                    "s": [{"c": control, "k": key, "m": modifier, "r": repeat, "rp": repeat}],
                    "t": 0,
                });
                self.bulk_write_messages(vec![json!({"hidmap": [entry]})]);
            }
        }
    }

    // -- inbound message processing --

    fn process_bulk_message(&self, message: &Value) {
        if self.debug {
            eprintln!(
                "[{:>6}] [recv] {message}",
                self.started_at.elapsed().as_millis()
            );
        }
        if let Some(layout) = message.get("l") {
            self.inner.lock().unwrap().layout_time = message.get("l_time").cloned();
            self.process_layout(layout.clone());
        } else if !self.legacy() {
            if let Some(time) = message.get("l_time") {
                let changed = self.inner.lock().unwrap().layout_time.as_ref() != Some(time);
                if changed {
                    self.bulk_write_messages(vec![json!({"get_layout": true})]);
                }
            }
        }
        // Layout-clock notifications may share a message with input events.
        if let Some(inputs) = message.get("in") {
            self.process_input(inputs);
        } else if let Some(version) = message.get("version_core").and_then(Value::as_str) {
            self.inner.lock().unwrap().version_core = version.to_string();
        }
    }

    /// rotate the base orientation and re-traverse the retained layout tree.
    fn rotate(&self, rotation: i32) {
        let layout_dict = {
            let mut inner = self.inner.lock().unwrap();
            let orientation = (inner.layout.base_orientation + rotation).rem_euclid(360);
            inner.config.profile.orientation = orientation;
            inner.layout.base_orientation = orientation;
            inner.layout.layout_dict.clone()
        };
        self.process_layout(layout_dict);
    }

    fn process_layout(&self, layout: Value) {
        {
            let mut inner = self.inner.lock().unwrap();
            inner.layout.update(&layout);
            // a connecting device replays each slider's resting/settling value
            // over several frames; key the echo-suppression window to each layout
            // so those do not echo as if the user moved the slider.
            inner.echo_live_at = Instant::now() + ECHO_SETTLE;
        }
        self.clear();
        self.cycle();
        self.configure_component_callbacks();
        self.configure_orientation();
        self.configure_leds(None, None, None);
        self.configure_component_hidkeys();
        self.configure_display();
        if self.map_midi {
            self.configure_midimap();
        }
    }

    fn process_input(&self, inputs: &Value) {
        let Some(inputs) = inputs.as_array() else {
            return;
        };
        for input_dict in inputs {
            let (Some(index), Some(values)) = (
                input_dict.get("i").and_then(Value::as_i64),
                input_dict.get("v").and_then(Value::as_array),
            ) else {
                continue;
            };
            let mut v = [0i64; 8];
            for (slot, value) in v.iter_mut().zip(values) {
                *slot = value.as_i64().unwrap_or(0);
            }

            let (events, hints, inspect_line, inspect_data) = {
                let mut inner = self.inner.lock().unwrap();
                let slider_broken = inner.version_core.as_str() < FIRMWARE_VERSION_SLIDER_FIXED;
                // a connecting device reports each slider's resting position and
                // its analog settling over several frames; suppress those echoes
                // until the settle window passes.
                let settling = Instant::now() < inner.echo_live_at;
                let Some(component) = inner.layout.find_component_mut(index) else {
                    continue;
                };
                // firmware <= 1.3.1 reports a slider maximum of 254, not 255.
                if slider_broken && component.kind == kind::SLIDER && v[0] == 254 {
                    v[0] = 255;
                }
                component.previous_values = component.values;
                component.values = v;
                // integrate the rotation deltas first so a `set` input reads the
                // up-to-date position; then decode and dispatch.
                component.accumulate();
                // the echo hints read the now-updated registers (joystick axes).
                let hints = EchoHints {
                    has_accumulator: matches!(
                        component.kind,
                        kind::DIAL | kind::TRI_DIAL | kind::ORBITER
                    ),
                    skip_baseline: settling && component.kind == kind::SLIDER,
                    joystick: (component.kind == kind::ORBITER).then(|| {
                        (
                            component.dominant_value(input::JOYSTICK_X),
                            component.dominant_value(input::JOYSTICK_Y),
                        )
                    }),
                };
                let inspect_line = self.inspect.then(|| component.inspect_line());
                let kind_name = component.kind_name();
                let mut events = Vec::new();
                let mut decoded: Vec<(&'static str, i64)> = Vec::new();
                for input_type in component.determine_input_types() {
                    let raw = component.dominant_value(input_type);
                    if self.inspect {
                        decoded.push((input_type, raw));
                    }
                    let outcome = component.invoke_callback(input_type);
                    events.push((kind_name, input_type, raw, outcome));
                }
                // long / double / triple press are derived from press timing and
                // dispatched additively to the raw pressed / released edges above.
                for input_type in component.press_events(Instant::now()) {
                    let raw = component.dominant_value(input_type);
                    let outcome = component.invoke_callback(input_type);
                    events.push((kind_name, input_type, raw, outcome));
                }
                let inspect_data = self.inspect.then_some((kind_name, decoded));
                (events, hints, inspect_line, inspect_data)
            };

            // inspect prints every input event (even one that decodes to
            // nothing), so an unrecognized control surfaces for follow-up, and
            // folds it into the deduplicated end-of-session summary.
            if let Some(line) = inspect_line {
                println!("{line}");
            }
            if let Some((kind_name, decoded)) = inspect_data {
                self.inspect_acc
                    .lock()
                    .unwrap()
                    .observe_input(index, kind_name, &v, &decoded);
            }

            // dispatch the bound action, then (in echo mode) show the input
            // value on the screen, matching upstream `display_input` ordering. a
            // dial reports both a rotation delta and its absolute `set` value in
            // one frame; echo only the accumulated `set` reading so a single turn
            // shows one value, not two (the rotation action still dispatches). the
            // orbiter pad's two axes collapse into one (x, y) line.
            let mut joystick_shown = false;
            for (kind_name, input_type, raw, outcome) in events {
                let display = match &outcome {
                    Some(o) => display_value(&o.value),
                    None => raw.to_string(),
                };
                if self.debug {
                    match &outcome {
                        Some(o) => eprintln!(
                            "[callback] fire {kind_name} {input_type} raw={raw} -> {} = {}",
                            o.action_name, display
                        ),
                        None => {
                            eprintln!("[callback] {kind_name} {input_type} raw={raw} (no callback)")
                        }
                    }
                }
                if let Some(outcome) = outcome {
                    self.dispatch_action(outcome);
                }
                if !self.echo
                    || hints.skip_baseline
                    || (hints.has_accumulator && input_type.starts_with("rotation"))
                {
                    continue;
                }
                if input_type.starts_with("joystick") {
                    if let (false, Some((x, y))) = (joystick_shown, hints.joystick) {
                        self.screen_string(format!("{kind_name}: {x},{y}"));
                        joystick_shown = true;
                    }
                    continue;
                }
                self.screen_string(format!("{kind_name}: {display}"));
            }
        }
    }

    // -- action dispatch (the callback set) --

    fn dispatch_action(&self, outcome: CallbackOutcome) {
        match outcome.action_name.as_str() {
            action::PROFILE_PREV => {
                self.inner.lock().unwrap().config.prev();
                self.start();
            }
            action::PROFILE_NEXT => {
                self.inner.lock().unwrap().config.next();
                self.start();
            }
            action::PROFILE_SWITCH => {
                let index = dispatch_number(&outcome.value) as usize;
                self.inner.lock().unwrap().config.switch(index);
                self.start();
            }
            action::SET_COLOR_RED => {
                let value = dispatch_number(&outcome.value) as i64;
                self.configure_leds(Some(value), None, None);
                self.screen_string(format!("Red: {value}"));
            }
            action::SET_COLOR_GREEN => {
                let value = dispatch_number(&outcome.value) as i64;
                self.configure_leds(None, Some(value), None);
                self.screen_string(format!("Green: {value}"));
            }
            action::SET_COLOR_BLUE => {
                let value = dispatch_number(&outcome.value) as i64;
                self.configure_leds(None, None, Some(value));
                self.screen_string(format!("Blue: {value}"));
            }
            action::EXECUTE_COMMAND => self.dispatch_execute_command(&outcome),
            action::CYCLE_MAPPING => self.dispatch_cycle_mapping(&outcome),
            action::TEST_CYCLE => self.screen_string(format!("C: {}", item_string(&outcome.value))),
            action::TEST_DELTA => self.screen_string(format!(
                "D: {}",
                format_number(dispatch_number(&outcome.value))
            )),
            action::TEST_ACCUMULATE => self.screen_string(format!(
                "A: {}",
                format_number(dispatch_number(&outcome.value))
            )),
            action::TEST_TRIGGER => self.screen_string(format!(
                "T: {}",
                format_number(dispatch_number(&outcome.value))
            )),
            action::TOGGLE_PREVIEW => {
                // toggle the base screen between the active profile's icon and a
                // blank screen (a preview of the profile's icon on / off).
                let icon = {
                    let mut inner = self.inner.lock().unwrap();
                    let profile_icon = inner.config.profile.icon;
                    let current = inner
                        .layout
                        .base_mut()
                        .map(|c| c.screen_display)
                        .unwrap_or(profile_icon);
                    if current == profile_icon {
                        icon::BLANK
                    } else {
                        profile_icon
                    }
                };
                self.screen_display(icon as i64);
            }
            _ => {}
        }
    }

    fn dispatch_execute_command(&self, outcome: &CallbackOutcome) {
        let Some(argv) = outcome.data.first().and_then(Value::as_array) else {
            return;
        };
        let value = format_number(dispatch_number(&outcome.value));
        let home = crate::resources::config_home();
        let args: Vec<String> = argv
            .iter()
            .map(|a| expand_command_arg(a.as_str().unwrap_or_default(), &value, &home))
            .collect();
        let Some((program, rest)) = args.split_first() else {
            return;
        };
        // anchor a still-relative program path to the config home so bundled
        // profiles resolve regardless of the working directory.
        let program = resolve_program(program, &home);
        let output = std::process::Command::new(&program)
            .args(rest)
            .env("AMBIT_VALUE", &value)
            .env("SEXTANT_CONFIG", &home)
            .output();
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let result = stdout.lines().next().unwrap_or("").trim();
                if !result.is_empty() {
                    self.screen_string(result.to_string());
                }
            }
            // surface the failure instead of swallowing it: a missing script or
            // unset `${NAME}` is otherwise invisible.
            Err(err) => eprintln!("[executeCommand] {}: {err}", program.display()),
        }
    }

    fn dispatch_cycle_mapping(&self, outcome: &CallbackOutcome) {
        let DispatchValue::Item { index, item } = &outcome.value else {
            return;
        };
        let Some(target) = outcome.data.first().and_then(Value::as_str) else {
            return;
        };
        {
            let mut inner = self.inner.lock().unwrap();
            let Some((uid, component_index)) = inner
                .layout
                .query(target)
                .first()
                .map(|c| (c.uid.clone(), c.index))
            else {
                return;
            };
            inner.config.set_component_actions(&uid, item.clone());
            self.set_component_callbacks(&mut inner, &uid, component_index);
        }
        let label = ["Red", "Green", "Blue"]
            .get(*index as usize)
            .copied()
            .unwrap_or_default();
        self.screen_string(label.to_string());
    }

    // -- worker threads --

    fn bulk_read_worker(&self) {
        while !self.is_shutdown() {
            for message in self.bulk_read() {
                self.process_bulk_message(&message);
            }
        }
    }

    /// the single chokepoint for every bulk-out transfer: it logs the transfer
    /// (in debug), advances the keepalive clock, and folds usb errors into one
    /// place. all device-facing output -- queued config / led / screen writes and
    /// the direct image / display-strip uploads -- funnels through here, so a
    /// unified write rate can be enforced in exactly one location.
    fn bulk_transfer(&self, data: &[u8]) {
        if self.debug {
            eprintln!(
                "[{:>6}] [bulk] {} bytes",
                self.started_at.elapsed().as_millis(),
                data.len()
            );
        }
        match self.handle.bulk_write(device::ENDPOINT_BULK_OUT, data) {
            Ok(_) => self.inner.lock().unwrap().last_write_time = Instant::now(),
            Err(err) => {
                self.inner.lock().unwrap().failed_writes += 1;
                self.process_usb_error(&err);
            }
        }
    }

    fn bulk_write_worker(&self) {
        while !self.is_shutdown() {
            match self.write_rx.recv_timeout(QUEUE_TIMEOUT) {
                Ok(data) => self.bulk_transfer(&data),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn led_worker(&self) {
        while !self.is_shutdown() {
            if self.led_rx.len() >= LED_QUEUE_DEPTH {
                self.drain_overflow(&self.led_rx, |inner| inner.dropped_leds += 1);
            }
            self.rate_limit(|inner| inner.last_led_time, LED_DELAY);
            match self.led_rx.recv_timeout(QUEUE_TIMEOUT) {
                Ok(leds) => {
                    let messages = self.led_messages(&leds);
                    self.bulk_write_messages(messages);
                    let mut inner = self.inner.lock().unwrap();
                    for led in &leds {
                        if let Some(component) = inner.layout.find_component_mut(led.i) {
                            component.led = (led.r, led.g, led.b);
                        }
                    }
                    inner.last_led_time = Instant::now();
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn led_messages(&self, leds: &[Led]) -> Vec<Value> {
        if self.legacy() {
            return vec![json!({"led": leds.iter().map(|l| l.to_json()).collect::<Vec<_>>()})];
        }
        // monogram cc addresses each led module individually with set_module;
        // the tri-dial carries three sub-leds, so it is keyed by component kind
        // (not a hardcoded index) so it works wherever the tri-dial attaches.
        let inner = self.inner.lock().unwrap();
        let mut messages = Vec::new();
        for led in leds {
            let has_three_leds = inner.layout.find_component(led.i).is_some_and(|c| {
                matches!(c.kind, kind::TRI_DIAL | kind::TRI_BUTTON | kind::TRI_SLIDER)
            });
            if has_three_leds {
                for sub in [0, 1, 2] {
                    messages.push(json!({"set_module": [led.i, 5, profile_module_color(&inner, sub as usize, led)]}));
                }
                messages.push(json!({"set_module": [led.i, 5, led_to_int(&[127, 0, 0, 0])]}));
            } else {
                messages
                    .push(json!({"set_module": [led.i, 2, profile_module_color(&inner, 0, led)]}));
            }
        }
        messages
    }

    fn screen_string_worker(&self) {
        while !self.is_shutdown() {
            self.rate_limit(|inner| inner.last_screen_string_time, SCREEN_STRING_DELAY);
            let mut title = match self.screen_rx.recv_timeout(QUEUE_TIMEOUT) {
                Ok(title) => title,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            // coalesce a burst to its most recent value, so a fast input (e.g.
            // the orbiter pad) shows its current reading rather than draining a
            // backlog of stale frames -- each monogram render is a full upload.
            let mut dropped = 0;
            while let Ok(next) = self.screen_rx.try_recv() {
                title = next;
                dropped += 1;
            }
            if dropped > 0 {
                self.inner.lock().unwrap().dropped_screen_strings += dropped;
            }
            if self.legacy() {
                self.bulk_write_messages(vec![json!({"screen_string": title})]);
            } else {
                // the monogram has no device-side text; rasterize it.
                self.render_screen(&title);
            }
            let mut inner = self.inner.lock().unwrap();
            if let Some(component) = inner.layout.base_mut() {
                component.screen_string = title.clone();
            }
            inner.last_screen_string_time = Instant::now();
            inner.current_screen_string_value = title;
        }
    }

    fn keepalive_worker(&self) {
        while !self.is_shutdown() {
            let (last_write, last_screen, title) = {
                let inner = self.inner.lock().unwrap();
                (
                    inner.last_write_time,
                    inner.last_screen_string_time,
                    inner.config.profile.title.clone(),
                )
            };
            let now = Instant::now();
            if now.duration_since(last_screen) > SCREEN_RESET {
                self.screen_string(title);
            }
            let keepalive = if self.legacy() { KEEPALIVE_TIMEOUT } else { MONOGRAM_KEEPALIVE_TIMEOUT };
            if now.duration_since(last_write) > keepalive {
                self.check();
            }
            thread::sleep(DEFAULT_WAIT);
        }
    }

    /// drop all but two queued items, counting each drop.
    fn drain_overflow<T>(&self, rx: &Receiver<T>, mut count: impl FnMut(&mut Inner)) {
        let mut inner = self.inner.lock().unwrap();
        while rx.len() > 2 {
            if rx.try_recv().is_ok() {
                count(&mut inner);
            } else {
                break;
            }
        }
    }

    /// sleep just long enough to honor a per-output rate limit.
    fn rate_limit(&self, last: impl Fn(&Inner) -> Instant, delay: Duration) {
        let elapsed = {
            let inner = self.inner.lock().unwrap();
            Instant::now().duration_since(last(&inner))
        };
        if elapsed < delay {
            thread::sleep(delay - elapsed);
        }
    }
}

fn dispatch_number(value: &DispatchValue) -> f64 {
    match value {
        DispatchValue::Number(n) => *n,
        DispatchValue::Item { index, .. } => *index as f64,
    }
}

/// expand an `executeCommand` argument: `${SEXTANT_CONFIG}` -> the configuration
/// home, `${AMBIT_VALUE}` / `${SEXTANT_VALUE}` -> the live input value, and any
/// other `${NAME}` -> the process environment (empty if unset). the legacy
/// `%AMBIT_VALUE%` token is also substituted.
fn expand_command_arg(arg: &str, value: &str, home: &Path) -> String {
    let arg = arg.replace("%AMBIT_VALUE%", value);
    expand_braces(&arg, |name| match name {
        "SEXTANT_CONFIG" => home.to_string_lossy().into_owned(),
        "AMBIT_VALUE" | "SEXTANT_VALUE" => value.to_string(),
        other => std::env::var(other).unwrap_or_default(),
    })
}

/// substitute `${NAME}` tokens using `lookup`; an unterminated `${` is emitted
/// verbatim.
fn expand_braces(input: &str, mut lookup: impl FnMut(&str) -> String) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find('}') {
            Some(end) => {
                out.push_str(&lookup(&after[..end]));
                rest = &after[end + 1..];
            }
            None => {
                out.push_str("${");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// resolve an `executeCommand` program path. a bare command (no separator) is a
/// PATH lookup; an absolute path is used as-is; a relative path is anchored to
/// the config home so the bundled profiles resolve from any working directory.
fn resolve_program(program: &str, home: &Path) -> PathBuf {
    let path = Path::new(program);
    if path.is_absolute() || !program.contains('/') {
        path.to_path_buf()
    } else {
        home.join(path)
    }
}

/// render a callback value for the on-screen input echo.
fn display_value(value: &DispatchValue) -> String {
    match value {
        DispatchValue::Number(n) => format_number(*n),
        DispatchValue::Item { index, item } => format!("({index}, {item})"),
    }
}

/// format a callback value, rendering an integral float without a decimal so
/// `%s` / `%d` formatting matches the python output.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        (value as i64).to_string()
    } else {
        value.to_string()
    }
}

fn item_string(value: &DispatchValue) -> String {
    match value {
        DispatchValue::Item { item, .. } => item
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| item.to_string()),
        DispatchValue::Number(n) => format_number(*n),
    }
}

fn profile_module_color(inner: &Inner, sub: usize, led: &Led) -> i64 {
    let color = inner
        .layout
        .find_component(led.i)
        .and_then(|m| inner.app_led_colors.get(&(m.uid.clone(), sub)));
    if let Some(&(r, g, b)) = color {
        // Official CoreV2 SetColor: low byte red, then green, then blue.
        ((sub as i64) << 24) | (r as i64) | ((g as i64) << 8) | ((b as i64) << 16)
    } else {
        module_color(sub as i64, led)
    }
}

fn led_to_int(bytes: &[i64; 4]) -> i64 {
    bytes.iter().fold(0i64, |acc, &b| (acc << 8) | (b & 0xff))
}

/// pack a component led color for a monogram `set_module` write: the high byte
/// is the sub-led index (`0` for single-led modules), the low 24 bits are
/// `0x00rrggbb` as the device expects (red is the high color byte).
fn module_color(sub: i64, led: &Led) -> i64 {
    led_to_int(&[sub, led.r as i64, led.g as i64, led.b as i64])
}

/// the `strip_rows`-row strip offsets whose pixels differ from the previously
/// uploaded frame, so a redraw re-sends only the dirty bands. a missing previous
/// frame (first draw, or after a failed upload) reports every strip.
fn changed_strips(
    previous: Option<&[u8]>,
    frame: &[u8],
    dim: usize,
    strip_rows: usize,
) -> Vec<usize> {
    let stride = dim * 2;
    (0..dim)
        .step_by(strip_rows)
        .filter(|&y| {
            let (lo, hi) = (y * stride, (y + strip_rows) * stride);
            previous.is_none_or(|prev| prev.get(lo..hi) != Some(&frame[lo..hi]))
        })
        .collect()
}

/// a 240x240 big-endian rgb565 monogram core-screen frame with `text` rasterized
/// over it. `logo` is the optional rgb565 background (the brand icon slot's
/// image, itself now a black frame); without it the background is plain black.
/// `font` is the optional glyph table; without it no text is drawn. both come
/// from the configuration home, so an un-initialized home yields a black frame.
fn screen_frame(text: &str, logo: Option<&[u8]>, font: Option<&[u8]>) -> Vec<u8> {
    let size = SCREEN_DIM * SCREEN_DIM * 2;
    let mut frame = match logo {
        Some(bytes) if bytes.len() == size => bytes.to_vec(),
        _ => vec![0u8; size],
    };
    if let Some(font) = font {
        draw_text(&mut frame, font, text);
    }
    frame
}

/// rotate a 240x240 rgb565 frame clockwise by a base orientation (degrees), so
/// the physical core screen turns with the layout.
fn rotate_frame(src: &[u8], orientation: i32) -> Vec<u8> {
    let orientation = orientation.rem_euclid(360);
    if orientation == 0 {
        return src.to_vec();
    }
    let dim = SCREEN_DIM;
    let mut dst = vec![0u8; src.len()];
    for sy in 0..dim {
        for sx in 0..dim {
            let (dx, dy) = match orientation {
                90 => (dim - 1 - sy, sx),
                180 => (dim - 1 - sx, dim - 1 - sy),
                270 => (sy, dim - 1 - sx),
                _ => (sx, sy),
            };
            let (s, d) = ((sy * dim + sx) * 2, (dy * dim + dx) * 2);
            dst[d..d + 2].copy_from_slice(&src[s..s + 2]);
        }
    }
    dst
}

/// rasterize `text` into a 240x240 rgb565 frame: 8x8 glyphs (ascii 32..126),
/// scaled, horizontally centered near the bottom, in the status text color.
/// the glyph size in pixels for a line: the configured scale's size, shrunk
/// *continuously* so a long line (e.g. "Tri-Dial: 255") fits the screen width
/// without the jarring sudden step a coarse integer scale would give.
fn fit_glyph(len: usize) -> f32 {
    let max = (8 * SCREEN_TEXT_SCALE) as f32;
    if len == 0 {
        max
    } else {
        max.min(SCREEN_DIM as f32 / len as f32)
    }
}

fn draw_text(frame: &mut [u8], font: &[u8], text: &str) {
    let glyph = fit_glyph(text.len());
    let start = ((SCREEN_DIM as f32 - glyph * text.len() as f32) / 2.0).max(0.0);
    let [hi, lo] = SCREEN_TEXT_COLOR.to_be_bytes();
    for (i, ch) in text.chars().enumerate() {
        let code = ch as usize;
        if !(32..127).contains(&code) {
            continue;
        }
        let rows = &font[(code - 32) * 8..(code - 32) * 8 + 8];
        let left = start + i as f32 * glyph;
        let (x0, x1) = (left as usize, (left + glyph) as usize);
        // map each destination pixel back to its 8x8 source pixel (a fractional
        // nearest-neighbor scale), so any glyph size renders smoothly.
        for y in SCREEN_TEXT_TOP..(SCREEN_TEXT_TOP + glyph as usize).min(SCREEN_DIM) {
            let sy = ((y - SCREEN_TEXT_TOP) as f32 * 8.0 / glyph) as usize;
            if sy >= 8 {
                continue;
            }
            let bits = rows[sy];
            for x in x0..x1.min(SCREEN_DIM) {
                let sx = ((x - x0) as f32 * 8.0 / glyph) as usize;
                if sx < 8 && bits & (1 << (7 - sx)) != 0 {
                    let off = (y * SCREEN_DIM + x) * 2;
                    frame[off] = hi;
                    frame[off + 1] = lo;
                }
            }
        }
    }
}

/// hash a component uid down to a 7-bit midi note, matching upstream.
fn component_hash(uid: &str) -> i64 {
    let table: Vec<i64> = (0..128).collect();
    let tail = &uid[uid.len().min(1)..];
    let mut h = tail.chars().take(1).count() as i64 % 128;
    for c in tail.chars() {
        h = table[((h ^ c as i64) & 0x7f) as usize];
    }
    h
}

fn is_known_action(action_name: &str) -> bool {
    matches!(
        action_name,
        action::PROFILE_PREV
            | action::PROFILE_NEXT
            | action::PROFILE_SWITCH
            | action::SET_COLOR_RED
            | action::SET_COLOR_GREEN
            | action::SET_COLOR_BLUE
            | action::EXECUTE_COMMAND
            | action::CYCLE_MAPPING
            | action::TOGGLE_PREVIEW
            | action::TEST_CYCLE
            | action::TEST_DELTA
            | action::TEST_ACCUMULATE
            | action::TEST_TRIGGER
    )
}

/// build the default behavior for an action, then let its `.plp` config win.
fn make_action_behavior(
    action_config: &Value,
    component_kind: u8,
    input_type: &str,
    action_name: &str,
) -> Behavior {
    let mut behavior = Behavior {
        action_config: action_config.clone(),
        ..Default::default()
    };
    if let Some(default) = kind_default(component_kind) {
        behavior.overlay(&default);
    }
    if let Some(default) = input_default(input_type) {
        behavior.overlay(&default);
    }
    if let Some(default) = action_default(action_name) {
        behavior.overlay(&default);
    }
    behavior
}

fn kind_default(component_kind: u8) -> Option<Behavior> {
    (component_kind == kind::SLIDER).then(|| Behavior {
        kind: Some(BehaviorKind::Trigger),
        limits: Some((0, 255)),
        invert: Some(false),
        ..Default::default()
    })
}

fn input_default(input_type: &str) -> Option<Behavior> {
    let behavior = match input_type {
        input::PRESSED | input::RELEASED => Behavior {
            kind: Some(BehaviorKind::Trigger),
            ..Default::default()
        },
        input::ROTATION_LEFT => Behavior {
            kind: Some(BehaviorKind::Accumulate),
            invert: Some(true),
            ..Default::default()
        },
        input::ROTATION_RIGHT => Behavior {
            kind: Some(BehaviorKind::Accumulate),
            invert: Some(false),
            ..Default::default()
        },
        input::SET => Behavior {
            kind: Some(BehaviorKind::Trigger),
            invert: Some(false),
            limits: Some((0, 255)),
            ..Default::default()
        },
        _ => return None,
    };
    Some(behavior)
}

fn action_default(action_name: &str) -> Option<Behavior> {
    let color = || Behavior {
        limits: Some((0, 255)),
        default: Some(255.0),
        threshold: Some(8),
        ..Default::default()
    };
    let behavior = match action_name {
        action::SET_COLOR_RED | action::SET_COLOR_GREEN | action::SET_COLOR_BLUE => color(),
        action::CYCLE_MAPPING | action::TEST_CYCLE => Behavior {
            kind: Some(BehaviorKind::Cycle),
            loops: Some(true),
            nested: Some(true),
            ..Default::default()
        },
        action::PROFILE_NEXT | action::PROFILE_PREV | action::TEST_TRIGGER => Behavior {
            kind: Some(BehaviorKind::Trigger),
            ..Default::default()
        },
        action::PROFILE_SWITCH => Behavior {
            kind: Some(BehaviorKind::Cycle),
            loops: Some(true),
            ..Default::default()
        },
        action::TEST_DELTA => Behavior {
            kind: Some(BehaviorKind::Delta),
            ..Default::default()
        },
        action::TEST_ACCUMULATE => Behavior {
            kind: Some(BehaviorKind::Accumulate),
            ..Default::default()
        },
        _ => return None,
    };
    Some(behavior)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_command_arg_resolves_config_value_and_env() {
        let home = Path::new("/home/u/.config/sextant");
        assert_eq!(
            "/home/u/.config/sextant/scripts/hello.sh",
            expand_command_arg("${SEXTANT_CONFIG}/scripts/hello.sh", "7", home)
        );
        // both the brace and legacy tokens carry the input value.
        assert_eq!(
            "v=7 (7)",
            expand_command_arg("v=${AMBIT_VALUE} (%AMBIT_VALUE%)", "7", home)
        );
        // an unset variable expands to empty; an unterminated brace stays literal.
        assert_eq!("", expand_command_arg("${SEXTANT_UNSET_XYZ}", "7", home));
        assert_eq!("${oops", expand_command_arg("${oops", "7", home));
    }

    #[test]
    fn resolve_program_anchors_relative_paths_to_home() {
        let home = Path::new("/cfg/sextant");
        // a relative path is anchored to the home; absolute and bare stay put.
        assert_eq!(
            PathBuf::from("/cfg/sextant/./scripts/hello.sh"),
            resolve_program("./scripts/hello.sh", home)
        );
        assert_eq!(
            PathBuf::from("/usr/bin/env"),
            resolve_program("/usr/bin/env", home)
        );
        assert_eq!(
            PathBuf::from("playerctl"),
            resolve_program("playerctl", home)
        );
    }

    #[test]
    fn rotate_frame_turns_the_screen_clockwise() {
        let dim = SCREEN_DIM;
        let mut frame = vec![0u8; dim * dim * 2];
        frame[0] = 0xab; // mark the top-left pixel
        frame[1] = 0xcd;
        // 90 clockwise sends the top-left pixel to the top-right.
        let rotated = rotate_frame(&frame, 90);
        let top_right = (dim - 1) * 2;
        assert_eq!([0xab, 0xcd], [rotated[top_right], rotated[top_right + 1]]);
        // orientation 0 is a no-op.
        assert_eq!(frame, rotate_frame(&frame, 0));
    }

    #[test]
    fn screen_text_scale_fits_the_screen_width() {
        // short lines use the full glyph size; a long line shrinks to fit.
        assert_eq!((8 * SCREEN_TEXT_SCALE) as f32, fit_glyph(5));
        assert!(fit_glyph(20) * 20.0 <= SCREEN_DIM as f32);
        // the shrink is continuous -- no sudden halving at a length boundary.
        assert!((fit_glyph(15) - fit_glyph(16)).abs() < 2.0);
    }

    #[test]
    fn screen_frame_rasterizes_text_over_black() {
        // the font comes from the embedded seed here (runtime reads the home).
        let font = crate::resources::resource_bytes("assets/monogram/font8x8.bin").unwrap();
        let frame = screen_frame("HI", None, Some(&font));
        assert_eq!(SCREEN_DIM * SCREEN_DIM * 2, frame.len());
        // the status text color appears in the text band (text was rasterized).
        let band = &frame[SCREEN_TEXT_TOP * SCREEN_DIM * 2..];
        let color = SCREEN_TEXT_COLOR.to_be_bytes();
        assert!(band.chunks(2).any(|p| p == color), "text was drawn");
    }

    #[test]
    fn screen_frame_defaults_to_a_black_background() {
        // without a logo, the background is black: above the text band (where the
        // logo would otherwise sit) every pixel is zero.
        let font = crate::resources::resource_bytes("assets/monogram/font8x8.bin").unwrap();
        let frame = screen_frame("HI", None, Some(&font));
        let above_text = &frame[..SCREEN_TEXT_TOP * SCREEN_DIM * 2];
        assert!(above_text.iter().all(|&b| b == 0), "background is black");
    }

    #[test]
    fn changed_strips_reports_only_dirty_bands() {
        // a tiny 4x4 frame in 2-row strips: stride 8 bytes, strips at y=0, y=2.
        let (dim, rows) = (4, 2);
        let base = vec![0u8; dim * dim * 2];

        // no previous frame -> every strip uploads (first draw / post-failure).
        assert_eq!(vec![0, 2], changed_strips(None, &base, dim, rows));
        // an identical re-render uploads nothing.
        assert!(changed_strips(Some(&base), &base, dim, rows).is_empty());

        // a pixel changed in row 2 dirties only the second strip.
        let mut next = base.clone();
        next[2 * dim * 2 + 1] = 0xff;
        assert_eq!(vec![2], changed_strips(Some(&base), &next, dim, rows));

        // changes in both bands re-send both.
        next[0] = 0xff;
        assert_eq!(vec![0, 2], changed_strips(Some(&base), &next, dim, rows));
    }

    #[test]
    fn changed_strips_covers_a_full_240_frame() {
        // the real screen geometry: 240/4 = 60 strips when nothing is cached.
        let frame = vec![0u8; SCREEN_DIM * SCREEN_DIM * 2];
        assert_eq!(
            SCREEN_DIM / SCREEN_STRIP_ROWS,
            changed_strips(None, &frame, SCREEN_DIM, SCREEN_STRIP_ROWS).len()
        );
        // a one-strip text edit over the cached frame re-sends a single strip.
        let mut next = frame.clone();
        let off = SCREEN_TEXT_TOP * SCREEN_DIM * 2;
        next[off] ^= 0xff;
        let strips = changed_strips(Some(&frame), &next, SCREEN_DIM, SCREEN_STRIP_ROWS);
        assert_eq!(
            vec![SCREEN_TEXT_TOP - SCREEN_TEXT_TOP % SCREEN_STRIP_ROWS],
            strips
        );
    }

    #[test]
    fn monogram_led_color_packs_rrggbb() {
        // the device expects 0x00rrggbb (red is the high color byte). ground
        // truth from the official-app captures: a tri-dial red sub-led is
        // 0x01ff0000 and green is 0x0200ff00.
        let red = Led {
            i: 0,
            r: 255,
            g: 0,
            b: 0,
            m: 0,
        };
        assert_eq!(0xff0000, module_color(0, &red));
        let blue = Led {
            i: 0,
            r: 0,
            g: 0,
            b: 255,
            m: 0,
        };
        assert_eq!(0x0000ff, module_color(0, &blue));
        // the sub-led index rides the high byte for the tri-dial.
        let green = Led {
            i: 1,
            r: 0,
            g: 255,
            b: 0,
            m: 0,
        };
        assert_eq!(0x0200ff00, module_color(2, &green));
    }
}
