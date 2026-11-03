#![recursion_limit = "256"]
//! an in-memory device that emulates the usb control + bulk protocol.
//!
//! [`FakeDevice`] implements the core [`Device`] trait, so the real controller
//! runs against it unchanged. it advances through the same control-handshake and
//! bulk phases as hardware, replies to `start` with the connected layout, and
//! exposes input injectors (button press, dial turn, slider move) that feed
//! `in` messages back to the host once the device reaches its input phase. the
//! recorded device state (leds, screen string, screen images, message log)
//! mirrors the python fake so the behavioral tests can assert against it.

pub mod layouts;

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use serde_json::{json, Value};
use sextant_core::component::kind;
use sextant_core::device::{ControlData, Device, DeviceClass, DeviceHandle, UsbError};
use sextant_core::message::{self, MessageFormat};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TO_HOST_DEPTH: usize = 32;
const DELAY_SCREEN_STRING: Duration = Duration::from_millis(50); // 1/20s
const DELAY_LED: Duration = Duration::from_micros(16_666); // 1/60s
const DEFAULT_VERSION: &str = "1.3.1";

/// the device's position in the connection lifecycle. injectors only emit input
/// once the device reaches [`Phase::Input`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Control0,
    Control1,
    Control2,
    Control3,
    Control4,
    Control5,
    Bulk,
    Input,
}

impl Phase {
    fn next(self) -> Phase {
        match self {
            Phase::Control0 => Phase::Control1,
            Phase::Control1 => Phase::Control2,
            Phase::Control2 => Phase::Control3,
            Phase::Control3 => Phase::Control4,
            Phase::Control4 => Phase::Control5,
            Phase::Control5 => Phase::Bulk,
            other => other,
        }
    }
}

#[derive(Default)]
struct HandleState {
    messages: HashMap<String, Vec<Value>>,
    leds: HashMap<i64, (u8, u8, u8)>,
    screen_display: Option<i64>,
    screen_string: Option<String>,
    screen_images: HashMap<i64, Vec<u8>>,
    layout: Value,
}

/// the opened device handle: the bulk/control i/o surface plus recorded state.
pub struct FakeHandle {
    state: Mutex<HandleState>,
    phase: Mutex<Phase>,
    version: String,
    format: MessageFormat,
    to_host_tx: Sender<Vec<u8>>,
    to_host_rx: Receiver<Vec<u8>>,
    // the byte length of every bulk-out transfer, so tests can assert the host
    // never sends a transfer large enough to reset the palette core.
    transfer_sizes: Mutex<Vec<usize>>,
}

impl FakeHandle {
    fn new(layout: Value, version: String, format: MessageFormat) -> FakeHandle {
        let (to_host_tx, to_host_rx) = bounded(TO_HOST_DEPTH);
        FakeHandle {
            state: Mutex::new(HandleState {
                layout,
                ..Default::default()
            }),
            phase: Mutex::new(Phase::Control0),
            version,
            format,
            to_host_tx,
            to_host_rx,
            transfer_sizes: Mutex::new(Vec::new()),
        }
    }

    /// the byte length of every bulk-out transfer the host has sent.
    pub fn transfer_sizes(&self) -> Vec<usize> {
        self.transfer_sizes.lock().unwrap().clone()
    }

    pub fn phase(&self) -> Phase {
        *self.phase.lock().unwrap()
    }

    pub fn screen_string(&self) -> Option<String> {
        self.state.lock().unwrap().screen_string.clone()
    }

    pub fn screen_display(&self) -> Option<i64> {
        self.state.lock().unwrap().screen_display
    }

    pub fn leds(&self) -> HashMap<i64, (u8, u8, u8)> {
        self.state.lock().unwrap().leds.clone()
    }

    pub fn screen_images(&self) -> HashMap<i64, Vec<u8>> {
        self.state.lock().unwrap().screen_images.clone()
    }

    /// the values logged for a message type (e.g. how many `check`s arrived).
    pub fn messages(&self, key: &str) -> Vec<Value> {
        self.state
            .lock()
            .unwrap()
            .messages
            .get(key)
            .cloned()
            .unwrap_or_default()
    }

    /// the full message log, keyed by message type.
    pub fn all_messages(&self) -> HashMap<String, Vec<Value>> {
        self.state.lock().unwrap().messages.clone()
    }

    fn set_layout(&self, layout: Value) {
        self.state.lock().unwrap().layout = layout;
    }

    /// queue framed messages for the host to read, dropping oldest on overflow.
    fn push_to_host(&self, data: Vec<u8>) {
        while self.to_host_tx.is_full() {
            if self.to_host_rx.try_recv().is_err() {
                break;
            }
        }
        let _ = self.to_host_tx.try_send(data);
    }

    fn write_messages(&self, messages: Vec<Value>) {
        let data = message::encode(&messages, self.format, &[]);
        self.push_to_host(data);
    }

    fn write_layout(&self) {
        let layout = self.state.lock().unwrap().layout.clone();
        self.write_messages(vec![json!({ "l": layout })]);
    }

    /// process one host-to-device message, recording it and acting on it.
    fn process_message(&self, message: &Value, extra: &[u8]) {
        let Some(object) = message.as_object() else {
            return;
        };

        // mimic the device's processing latency before the state mutates.
        if object.contains_key("led") {
            std::thread::sleep(DELAY_LED);
        }
        if object.contains_key("screen_string") {
            std::thread::sleep(DELAY_SCREEN_STRING);
        }

        let mut state = self.state.lock().unwrap();
        for (key, value) in object {
            state
                .messages
                .entry(key.clone())
                .or_default()
                .push(value.clone());
        }
        if let Some(leds) = object.get("led").and_then(Value::as_array) {
            for led in leds {
                if let (Some(i), Some(r), Some(g), Some(b)) = (
                    led.get("i").and_then(Value::as_i64),
                    led.get("r").and_then(Value::as_u64),
                    led.get("g").and_then(Value::as_u64),
                    led.get("b").and_then(Value::as_u64),
                ) {
                    state.leds.insert(i, (r as u8, g as u8, b as u8));
                }
            }
        }
        if let Some(index) = object.get("screen_display").and_then(Value::as_i64) {
            state.screen_display = Some(index);
        }
        if let Some(title) = object.get("screen_string").and_then(Value::as_str) {
            state.screen_string = Some(title.to_string());
        }
        if let Some(index) = object.get("screen_write").and_then(Value::as_i64) {
            state.screen_images.insert(index, extra.to_vec());
        }
        if object.contains_key("stop") {
            *self.phase.lock().unwrap() = Phase::Bulk;
        }
        // palettegear asks for the layout with `start`; monogram with `get_layout`.
        let started = object.contains_key("start") || object.contains_key("get_layout");
        let send_version =
            object.contains_key("send_version") || object.contains_key("get_version");
        drop(state);

        // these reply on the device-to-host queue, so run them without the lock.
        if started {
            self.write_layout();
            let mut phase = self.phase.lock().unwrap();
            if *phase == Phase::Bulk {
                *phase = Phase::Input;
            }
        }
        if send_version {
            self.write_messages(vec![json!({
                "version_core": self.version,
                "version_screen": 0,
            })]);
        }
    }
}

impl DeviceHandle for FakeHandle {
    fn control(
        &self,
        _request_type: u8,
        _request: u8,
        _data: ControlData<'_>,
    ) -> Result<(), UsbError> {
        let mut phase = self.phase.lock().unwrap();
        *phase = phase.next();
        Ok(())
    }

    fn bulk_write(&self, _endpoint: u8, data: &[u8]) -> Result<usize, UsbError> {
        self.transfer_sizes.lock().unwrap().push(data.len());
        let (messages, extra) = message::decode(data, self.format);
        for message in &messages {
            self.process_message(message, &extra);
        }
        Ok(data.len())
    }

    fn bulk_read(
        &self,
        _endpoint: u8,
        _max: usize,
        timeout: Duration,
    ) -> Result<Vec<u8>, UsbError> {
        match self.to_host_rx.recv_timeout(timeout) {
            Ok(data) => Ok(data),
            Err(RecvTimeoutError::Timeout) => Err(UsbError::Timeout),
            Err(RecvTimeoutError::Disconnected) => Err(UsbError::Disconnected),
        }
    }

    fn reset(&self) {}
    fn release(&self) {}
}

#[derive(Default)]
struct DeviceState {
    layout: Value,
    values: HashMap<i64, [i64; 8]>,
    kinds: HashMap<i64, u8>,
}

fn initialize_values(node: &Value, state: &mut DeviceState) {
    let Some(index) = node.get("i").and_then(Value::as_i64) else {
        return;
    };
    state.values.insert(index, [0; 8]);
    if let Some(component_kind) = node.get("t").and_then(Value::as_u64) {
        state.kinds.insert(index, component_kind as u8);
    }
    if let Some(children) = node.get("c").and_then(Value::as_array) {
        for child in children {
            initialize_values(child, state);
        }
    }
}

/// the in-memory device. construct it, connect a layout, hand it to a controller,
/// then drive it with the input injectors.
pub struct FakeDevice {
    legacy: bool,
    version: String,
    state: Mutex<DeviceState>,
    handle: Mutex<Option<Arc<FakeHandle>>>,
}

impl FakeDevice {
    pub fn new(_device_id: &str) -> FakeDevice {
        FakeDevice {
            legacy: true,
            version: String::new(),
            state: Mutex::new(DeviceState::default()),
            handle: Mutex::new(None),
        }
    }

    pub fn set_legacy(&mut self, legacy: bool) {
        self.legacy = legacy;
    }

    /// the firmware version reported at handshake. has no effect after `open`.
    pub fn set_version_core(&mut self, version: &str) {
        self.version = version.to_string();
    }

    /// attach a layout, resetting the per-component value and kind tables.
    pub fn components_connected(&self, layout: Value) {
        let mut state = self.state.lock().unwrap();
        state.values.clear();
        state.kinds.clear();
        initialize_values(&layout, &mut state);
        state.layout = layout;
    }

    /// swap to a new layout after the device is open, notifying the host.
    pub fn layout_changed(&self, layout: Value) {
        self.components_connected(layout.clone());
        if let Some(handle) = self.handle.lock().unwrap().as_ref() {
            handle.set_layout(layout);
            handle.write_layout();
        }
    }

    /// the opened handle, for asserting recorded device state in tests.
    pub fn handle(&self) -> Arc<FakeHandle> {
        self.handle
            .lock()
            .unwrap()
            .clone()
            .expect("device is not open")
    }

    fn handle_phase(&self) -> Phase {
        self.handle
            .lock()
            .unwrap()
            .as_ref()
            .map(|h| h.phase())
            .unwrap_or(Phase::Control0)
    }

    /// whether the component at `index` is one of `allowed`; an empty `allowed`
    /// matches any connected component (used by the generic `input` injector).
    fn is_kind(&self, index: i64, allowed: &[u8]) -> bool {
        self.state
            .lock()
            .unwrap()
            .kinds
            .get(&index)
            .is_some_and(|k| allowed.is_empty() || allowed.contains(k))
    }

    fn emit(&self, index: i64, values: [i64; 8]) {
        if let Some(handle) = self.handle.lock().unwrap().as_ref() {
            let values: Vec<i64> = values.to_vec();
            handle.write_messages(vec![json!({"in": [{"i": index, "v": values}]})]);
        }
    }

    /// run a closure against a component's value register and emit the result,
    /// but only while the device is accepting input.
    fn with_values(&self, index: i64, allowed: &[u8], update: impl FnOnce(&mut [i64; 8]) -> bool) {
        if self.handle_phase() != Phase::Input || !self.is_kind(index, allowed) {
            return;
        }
        let snapshot = {
            let mut state = self.state.lock().unwrap();
            let Some(values) = state.values.get_mut(&index) else {
                return;
            };
            if !update(values) {
                return;
            }
            *values
        };
        self.emit(index, snapshot);
    }

    pub fn input_pressed(&self, index: i64) {
        self.with_values(index, &[kind::DIAL, kind::BUTTON], |v| {
            v[0] = 1;
            v[1] = 0;
            v[2] = 0;
            true
        });
    }

    pub fn input_released(&self, index: i64) {
        self.with_values(index, &[kind::DIAL, kind::BUTTON], |v| {
            v[0] = 0;
            v[1] = 0;
            v[2] = 0;
            true
        });
    }

    pub fn input_slide_up(&self, index: i64, amount: i64) {
        let max = self.slider_max();
        self.with_values(index, &[kind::SLIDER], |v| {
            if v[0] == max {
                return false;
            }
            v[0] = (v[0] + amount).min(max);
            true
        });
    }

    pub fn input_slide_down(&self, index: i64, amount: i64) {
        self.with_values(index, &[kind::SLIDER], |v| {
            if v[0] == 0 {
                return false;
            }
            v[0] = (v[0] - amount).max(0);
            true
        });
    }

    pub fn input_rotation_left(&self, index: i64, amount: i64) {
        self.with_values(index, &[kind::DIAL], |v| {
            v[1] = amount;
            v[2] = 0;
            v[3] = (v[3] - amount).max(0);
            true
        });
    }

    pub fn input_rotation_right(&self, index: i64, amount: i64) {
        self.with_values(index, &[kind::DIAL], |v| {
            v[1] = 0;
            v[2] = amount;
            v[3] = (v[3] + amount).min(255);
            true
        });
    }

    /// inject a raw 8-register input event for any component, for modules without
    /// a dedicated injector (the monogram tri-dial / orbiter / core).
    pub fn input(&self, index: i64, values: [i64; 8]) {
        self.with_values(index, &[], |v| {
            *v = values;
            true
        });
    }

    /// firmware before 1.3.2 caps sliders at 254 instead of 255.
    fn slider_max(&self) -> i64 {
        let broken = self
            .handle
            .lock()
            .unwrap()
            .as_ref()
            .map(|h| h.version.as_str() < "1.3.2")
            .unwrap_or(false);
        if broken {
            254
        } else {
            255
        }
    }
}

impl Device for FakeDevice {
    fn class(&self) -> DeviceClass {
        if self.legacy {
            DeviceClass::PaletteGear
        } else {
            DeviceClass::MonogramCc
        }
    }

    fn open(&mut self) -> Result<Arc<dyn DeviceHandle>, UsbError> {
        let layout = self.state.lock().unwrap().layout.clone();
        let version = if self.version.is_empty() {
            DEFAULT_VERSION.to_string()
        } else {
            self.version.clone()
        };
        let format = if self.legacy {
            MessageFormat::Json
        } else {
            MessageFormat::Msgpack
        };
        let handle = Arc::new(FakeHandle::new(layout, version, format));
        *self.handle.lock().unwrap() = Some(handle.clone());
        Ok(handle)
    }
}
