//! the component data model, behavior engine, and layout graph.
//!
//! a [`Component`] is one physical module (base, button, dial, slider, ...) with
//! eight value registers updated from device input. a [`ComponentLayout`] is the
//! connected graph built by traversing the device's reported tree, assigning
//! each component a grid slot and orientation. the behavior engine ([`Behavior`]
//! and [`Invocation`]) turns raw value changes into discrete callback values per
//! the cycle / trigger / accumulate / delta semantics.

use crate::configuration::input;
use crate::coordinates::{calculate_female_slots, calculate_female_slots_wide, Orientation, Slot};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

/// component kind identifiers, as reported by the device in the layout tree.
pub mod kind {
    pub const BASE: u8 = 0;
    pub const BUTTON: u8 = 1;
    pub const DIAL: u8 = 2;
    pub const SLIDER: u8 = 3;
    pub const TRI_BUTTON: u8 = 8;
    pub const TRI_DIAL: u8 = 9;
    pub const TRI_SLIDER: u8 = 10;
    pub const ORBITER: u8 = 24;
    pub const CORE: u8 = 25;
}

/// how many 0-255 steps one rotation detent advances an accumulated dial value,
/// so a continuous knob sweeps its range in a usable number of turns.
const ROTATION_SENSITIVITY: i64 = 4;

/// a button / dial held at least this long registers a long press on release.
const LONG_PRESS: Duration = Duration::from_millis(500);
/// presses within this window of each other count as a double / triple press.
const MULTI_TAP: Duration = Duration::from_millis(300);

/// display name for a kind; the inverse is [`kind_from_name`].
pub fn kind_name(kind: u8) -> &'static str {
    match kind {
        kind::BASE => "Base",
        kind::BUTTON => "Button",
        kind::DIAL => "Dial",
        kind::SLIDER => "Slider",
        kind::TRI_BUTTON => "Tri-Button",
        kind::TRI_DIAL => "Tri-Dial",
        kind::TRI_SLIDER => "Tri-Slider",
        kind::CORE => "Core",
        kind::ORBITER => "Orbiter",
        _ => "Unknown",
    }
}

/// resolve a kind display name (as used in a layout query) to its identifier.
pub fn kind_from_name(name: &str) -> Option<u8> {
    match name {
        "Base" => Some(kind::BASE),
        "Button" => Some(kind::BUTTON),
        "Dial" => Some(kind::DIAL),
        "Slider" => Some(kind::SLIDER),
        "Tri-Button" => Some(kind::TRI_BUTTON),
        "Tri-Dial" => Some(kind::TRI_DIAL),
        "Tri-Slider" => Some(kind::TRI_SLIDER),
        "Core" => Some(kind::CORE),
        "Orbiter" => Some(kind::ORBITER),
        _ => None,
    }
}

/// the cell footprint `(width, height)` a module occupies in the layout grid.
/// most modules are 1x1; the wide slider and the monogram modules span more.
pub fn footprint(kind: u8) -> (i32, i32) {
    match kind {
        kind::SLIDER => (2, 1),
        kind::CORE => (2, 1),
        kind::TRI_DIAL => (1, 2),
        kind::ORBITER => (2, 2),
        _ => (1, 1),
    }
}

/// the layout cells a monogram module occupies at orientation 0 -- the device's
/// screen-upright pose: the core is vertical on the right (screen above buttons),
/// the orbiter is the 2x2 to its upper-left, and the tri-dial is below the
/// orbiter. (the device only reports a linear chain, so this is fixed here.)
fn monogram_cells(kind: u8) -> Vec<Slot> {
    match kind {
        kind::CORE => vec![(0, 0), (0, -1)],
        kind::ORBITER => vec![(-2, 0), (-2, 1), (-1, 0), (-1, 1)],
        kind::TRI_DIAL => vec![(-2, -1), (-1, -1)],
        _ => Vec::new(),
    }
}

/// rotate a layout cell about the origin by a base orientation (degrees), so the
/// whole monogram setup turns with the layout.
fn rotate_slot((x, y): Slot, orientation: Orientation) -> Slot {
    match orientation.rem_euclid(360) {
        90 => (y, -x),
        180 => (-x, -y),
        270 => (-y, x),
        _ => (x, y),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BehaviorKind {
    Cycle,
    Trigger,
    Accumulate,
    Delta,
}

impl BehaviorKind {
    pub fn name(self) -> &'static str {
        match self {
            BehaviorKind::Cycle => "CYCLE",
            BehaviorKind::Trigger => "TRIGGER",
            BehaviorKind::Accumulate => "ACCUMULATE",
            BehaviorKind::Delta => "DELTA",
        }
    }
}

/// a configured behavior: how a component's raw input is mapped to discrete
/// callback values. fields left unset fall back to the defaults applied by
/// [`Behavior::finalize`], so the engine reads them through accessors.
#[derive(Clone, Debug, Default)]
pub struct Behavior {
    pub kind: Option<BehaviorKind>,
    pub items: Vec<Value>,
    pub limits: Option<(i64, i64)>,
    pub loops: Option<bool>,
    pub invert: Option<bool>,
    pub nested: Option<bool>,
    pub discrete: Option<bool>,
    pub threshold: Option<i64>,
    pub value: Option<f64>,
    pub default: Option<f64>,
    /// the raw `.plp` binding config, re-applied last by [`Behavior::finalize`]
    /// so it overrides every default.
    pub action_config: Value,
}

impl Behavior {
    pub fn new(kind: BehaviorKind) -> Behavior {
        Behavior {
            kind: Some(kind),
            ..Default::default()
        }
    }

    fn invert(&self) -> bool {
        self.invert.unwrap_or(false)
    }

    fn loops(&self) -> bool {
        self.loops.unwrap_or(false)
    }

    fn discrete(&self) -> bool {
        self.discrete.unwrap_or(true)
    }

    fn nested(&self) -> bool {
        self.nested.unwrap_or(false)
    }

    fn threshold(&self) -> i64 {
        match self.threshold {
            Some(t) if t != 0 => t,
            _ => 1,
        }
    }

    fn default_value(&self) -> f64 {
        self.default.unwrap_or(0.0)
    }

    /// setting items derives the cycle limits from their count.
    pub fn set_items(&mut self, items: Vec<Value>) {
        self.items = items;
        if !self.items.is_empty() {
            self.limits = Some((0, self.items.len() as i64 - 1));
        }
    }

    /// derive cycle limits from a count without carrying the items themselves
    /// (used for profile switching, where the items are profiles).
    pub fn set_item_count(&mut self, count: usize) {
        if count > 0 {
            self.limits = Some((0, count as i64 - 1));
        }
    }

    /// whether a value falls outside the configured limits.
    pub fn outside(&self, value: f64) -> bool {
        match self.limits {
            Some((l0, l1)) => value < l0 as f64 || value > l1 as f64,
            None => false,
        }
    }

    /// overlay another behavior's set fields onto this one (the other wins),
    /// concatenating any items. mirrors the upstream `__add__` accumulation.
    pub fn overlay(&mut self, other: &Behavior) {
        if other.kind.is_some() {
            self.kind = other.kind;
        }
        if other.limits.is_some() {
            self.limits = other.limits;
        }
        if !other.items.is_empty() {
            let mut items = std::mem::take(&mut self.items);
            items.extend(other.items.iter().cloned());
            self.set_items(items);
        }
        for (dst, src) in [
            (&mut self.loops, other.loops),
            (&mut self.invert, other.invert),
            (&mut self.nested, other.nested),
            (&mut self.discrete, other.discrete),
        ] {
            if src.is_some() {
                *dst = src;
            }
        }
        if other.threshold.is_some() {
            self.threshold = other.threshold;
        }
        if other.value.is_some() {
            self.value = other.value;
        }
        if other.default.is_some() {
            self.default = other.default;
        }
    }

    /// fill in defaults that the engine assumes are concrete, then re-apply the
    /// `.plp` binding config last so it overrides every default.
    pub fn finalize(&mut self) {
        if self.default.is_none() || self.default == Some(0.0) {
            self.default = Some(0.0);
        }
        if self.discrete.is_none() {
            self.discrete = Some(true);
        }
        if self.threshold.unwrap_or(0) == 0 {
            self.threshold = Some(1);
        }
        let items = std::mem::take(&mut self.items);
        self.set_items(items);
        let config = self.action_config.clone();
        self.merge_config(&config);
    }

    /// overlay the json `actionMap` binding config onto this behavior.
    pub fn merge_config(&mut self, config: &Value) {
        let Some(map) = config.as_object() else {
            return;
        };
        if let Some(v) = map.get("value").and_then(Value::as_f64) {
            self.value = Some(v);
        }
        if let Some(v) = map.get("invert").and_then(Value::as_bool) {
            self.invert = Some(v);
        }
        if let Some(v) = map.get("loop").and_then(Value::as_bool) {
            self.loops = Some(v);
        }
        if let Some(v) = map.get("default").and_then(Value::as_f64) {
            self.default = Some(v);
        }
        if let Some(v) = map.get("discrete").and_then(Value::as_bool) {
            self.discrete = Some(v);
        }
        if let Some(items) = map.get("items").and_then(Value::as_array) {
            self.set_items(items.clone());
        }
        if let Some(limits) = map.get("limits").and_then(Value::as_array) {
            if limits.len() == 2 {
                self.limits = Some((
                    limits[0].as_i64().unwrap_or(0),
                    limits[1].as_i64().unwrap_or(0),
                ));
            } else {
                self.limits = None;
            }
        }
        if let Some(v) = map.get("threshold").and_then(Value::as_i64) {
            self.threshold = Some(v);
        }
    }
}

/// the value handed to a bound callback: a number, or (for a "nested" cycle
/// behavior) the selected index together with its item value.
#[derive(Clone, Debug, PartialEq)]
pub enum DispatchValue {
    Number(f64),
    Item { index: i64, item: Value },
}

/// the result of a fired binding: the action to dispatch, its value, and any
/// extra data the action needs (an argv for execute-command, a query target for
/// cycle-mapping).
#[derive(Clone, Debug)]
pub struct CallbackOutcome {
    pub action_name: String,
    pub value: DispatchValue,
    pub data: Vec<Value>,
}

/// one input binding on a component: a behavior plus the per-binding running
/// state (the cumulative change toward the threshold gate).
#[derive(Clone, Debug)]
pub struct Invocation {
    pub input_type: String,
    pub action_name: String,
    pub behavior: Behavior,
    pub vindex: usize,
    pub threshold: i64,
    pub cumulative_change: i64,
    pub data: Vec<Value>,
}

impl Invocation {
    pub fn new(
        input_type: &str,
        action_name: &str,
        mut behavior: Behavior,
        vindex: usize,
        data: Vec<Value>,
    ) -> Invocation {
        behavior.finalize();
        let threshold = behavior.threshold();
        Invocation {
            input_type: input_type.to_string(),
            action_name: action_name.to_string(),
            behavior,
            vindex,
            threshold,
            cumulative_change: 0,
            data,
        }
    }

    /// the default stored value a fresh binding seeds its component with.
    pub fn default_value(&self) -> f64 {
        self.behavior.default_value()
    }

    /// the discrete callback value implied by a raw input value and the
    /// component's stored value, before the threshold gate.
    pub fn callback_value(&self, value: i64, stored_value: f64) -> f64 {
        let b = &self.behavior;
        let mut cb = value as f64;
        let mut change = self.cumulative_change as f64;

        if b.kind == Some(BehaviorKind::Cycle) {
            change = 1.0;
        }
        if b.invert() {
            change = -change;
        }

        if b.kind == Some(BehaviorKind::Trigger) {
            cb = value as f64;
            if self.base_input() == input::SET {
                if let Some((l0, l1)) = b.limits {
                    let step = 255.0 / (l1 - l0) as f64;
                    cb = l0 as f64 + value as f64 / step;
                }
            }
        }

        if b.kind == Some(BehaviorKind::Cycle) {
            cb = stored_value + change;
            if self.base_input() == input::SET {
                if let Some((l0, l1)) = b.limits {
                    let step = 255.0 / (l1 - l0) as f64;
                    cb = (value as f64 / step).trunc();
                }
            }
        }

        if b.kind == Some(BehaviorKind::Delta) {
            cb = self.cumulative_change as f64;
            if b.invert() {
                cb = -cb;
            }
        }

        if let Some(v) = b.value {
            cb = v;
        }

        if b.kind == Some(BehaviorKind::Accumulate) {
            change = self.threshold as f64;
            if let Some(v) = b.value {
                if v != 0.0 {
                    change = v;
                }
            }
            if b.invert() {
                change = -change;
            }
            cb = stored_value + change;
        }

        if b.loops() {
            if let Some((l0, l1)) = b.limits {
                if cb < l0 as f64 {
                    cb = l1 as f64;
                }
                if cb > l1 as f64 {
                    cb = l0 as f64;
                }
            }
        } else if b.outside(cb) {
            if let Some((l0, l1)) = b.limits {
                if cb < l0 as f64 {
                    cb = l0 as f64;
                }
                if cb > l1 as f64 {
                    cb = l1 as f64;
                }
            }
        }

        if b.discrete() {
            cb = cb.trunc();
        }
        cb
    }

    /// the base input type, with any `_<n>` sub-control suffix stripped.
    fn base_input(&self) -> &str {
        split_index(&self.input_type).0
    }

    /// the unit of change contributed by one input event.
    pub fn value_delta(&self, value: i64, previous_value: i64) -> i64 {
        match self.base_input() {
            input::ROTATION_LEFT | input::ROTATION_RIGHT => value.abs(),
            input::SET => (value - previous_value).abs(),
            input::PRESSED
            | input::RELEASED
            | input::LONG_PRESSED
            | input::DOUBLE_PRESSED
            | input::TRIPLE_PRESSED => 1,
            _ => 0,
        }
    }

    /// a trigger landing exactly on a limit always fires, bypassing the gate.
    fn mandatory(&self, value: f64) -> bool {
        if self.behavior.kind != Some(BehaviorKind::Trigger) {
            return false;
        }
        match self.behavior.limits {
            Some((l0, l1)) => value == l0 as f64 || value == l1 as f64,
            None => false,
        }
    }

    /// a cycle / accumulate value equal to the stored value is dropped.
    fn redundant(&self, value: f64, stored_value: f64) -> bool {
        matches!(
            self.behavior.kind,
            Some(BehaviorKind::Cycle) | Some(BehaviorKind::Accumulate)
        ) && value == stored_value
    }

    /// fold one input event into the binding, returning the dispatch value when
    /// the threshold gate fires. updates the shared `stored_value` in place.
    pub fn run(
        &mut self,
        value: i64,
        previous_value: i64,
        stored_value: &mut f64,
    ) -> Option<DispatchValue> {
        self.cumulative_change += self.value_delta(value, previous_value);
        let cb = self.callback_value(value, *stored_value);

        if !self.mandatory(cb) && self.cumulative_change < self.threshold {
            return None;
        }
        if self.redundant(cb, *stored_value) {
            return None;
        }

        self.cumulative_change = 0;
        *stored_value = cb;

        if self.behavior.nested() {
            let index = cb as i64;
            let item = self
                .behavior
                .items
                .get(index.max(0) as usize)
                .cloned()
                .unwrap_or_default();
            Some(DispatchValue::Item { index, item })
        } else {
            Some(DispatchValue::Number(cb))
        }
    }
}

/// one physical module on the device.
#[derive(Clone, Debug)]
pub struct Component {
    pub index: i64,
    pub uid: String,
    pub kind: u8,
    pub flip: bool,
    pub screen_string: String,
    pub screen_display: u8,
    pub values: [i64; 8],
    pub previous_values: [i64; 8],
    /// a persistent 0-255 position per register, integrated from rotation
    /// deltas, so a delta-only control (orbiter, tri-dial knob) presents a stable
    /// "dial" value -- exposed as a `set` input and readable by a polling
    /// consumer (the demoscene). `previous_accumulated` holds the prior frame's
    /// value so the `set` behavior can compute its change.
    pub accumulated: [i64; 8],
    pub previous_accumulated: [i64; 8],
    /// the cell footprint `(width, height)` the module occupies; defaults to the
    /// kind's footprint, but the monogram placement rotates it with the layout.
    pub footprint: (i32, i32),
    pub led: (u8, u8, u8),
    pub sensitivity: i64,
    pub num_ports: Option<usize>,
    pub slot: Option<Slot>,
    pub orientation: Option<Orientation>,
    pub parent_uid: Option<String>,
    pub parent_port: Option<usize>,
    pub children: HashMap<usize, String>,
    /// per-component cycle/accumulate state, shared across all of its bindings.
    pub stored_value: f64,
    stored_value_seeded: bool,
    // multi-press timing (button / dial): the current hold's start (for a long
    // press on release), the last press time and the tap streak (for double /
    // triple press within a window).
    press_started: Option<Instant>,
    last_press: Option<Instant>,
    press_streak: i64,
    callbacks: HashMap<String, Invocation>,
    pub hidkeys: Vec<HidKey>,
}

/// a programmed hid binding on a component: control, key, modifier, repeat.
pub type HidKey = (String, u8, u8, u8);

impl Component {
    pub fn new(index: i64, uid: &str, kind: u8) -> Component {
        Component {
            index,
            uid: uid.to_string(),
            kind,
            flip: false,
            screen_string: String::new(),
            screen_display: crate::configuration::icon::BLANK,
            values: [0; 8],
            previous_values: [0; 8],
            accumulated: [0; 8],
            previous_accumulated: [0; 8],
            footprint: footprint(kind),
            led: (255, 255, 255),
            sensitivity: 9,
            num_ports: None,
            slot: None,
            orientation: None,
            parent_uid: None,
            parent_port: None,
            children: HashMap::new(),
            stored_value: 0.0,
            stored_value_seeded: false,
            press_started: None,
            last_press: None,
            press_streak: 0,
            callbacks: HashMap::new(),
            hidkeys: Vec::new(),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        kind_name(self.kind)
    }

    /// clear bound callbacks and hid keys; the stored value persists, matching
    /// upstream where the persistent state outlives a rebind.
    pub fn reset(&mut self) {
        self.callbacks.clear();
        self.hidkeys.clear();
        self.press_started = None;
        self.last_press = None;
        self.press_streak = 0;
    }

    /// bind a behavior to an input event. the first binding on a component seeds
    /// its stored value from the behavior default.
    pub fn set_callback(
        &mut self,
        input_type: &str,
        action_name: &str,
        behavior: Behavior,
        data: Vec<Value>,
    ) {
        let vindex = self.determine_vindex(input_type);
        let invocation = Invocation::new(input_type, action_name, behavior, vindex, data);
        if !self.stored_value_seeded {
            self.stored_value = invocation.default_value();
            self.stored_value_seeded = true;
        }
        self.callbacks.insert(input_type.to_string(), invocation);
    }

    pub fn set_hidkey(&mut self, control: &str, key: u8, modifier: u8, repeat: u8) {
        self.hidkeys
            .push((control.to_string(), key, modifier, repeat));
    }

    /// the behavior kind of any bound callback, for the simulator's label.
    pub fn behavior_label(&self) -> Option<&'static str> {
        self.callbacks
            .values()
            .find_map(|invocation| invocation.behavior.kind.map(BehaviorKind::name))
    }

    /// run the binding for an input event against the current value registers,
    /// returning the outcome to dispatch when the threshold gate fires.
    pub fn invoke_callback(&mut self, input_type: &str) -> Option<CallbackOutcome> {
        if !self.callbacks.contains_key(input_type) {
            return None;
        }
        let vindex = self.determine_vindex(input_type);
        // a synthesized orbiter / tri-dial `set` reads the integrated position
        // and its prior value, not the raw (delta) register.
        let (value, previous) = if split_index(input_type).0 == input::SET
            && matches!(self.kind, kind::ORBITER | kind::TRI_DIAL)
        {
            (self.accumulated[vindex], self.previous_accumulated[vindex])
        } else {
            (self.values[vindex], self.previous_values[vindex])
        };
        // take the invocation out so its run can borrow the stored value.
        let mut invocation = self.callbacks.remove(input_type).unwrap();
        let result = invocation.run(value, previous, &mut self.stored_value);
        let outcome = result.map(|value| CallbackOutcome {
            action_name: invocation.action_name.clone(),
            value,
            data: invocation.data.clone(),
        });
        self.callbacks.insert(input_type.to_string(), invocation);
        outcome
    }

    pub fn set_sensitivity(&mut self, sensitivity: i64) {
        self.sensitivity = sensitivity;
    }

    /// sliders mounted at 90 or 180 degrees read their travel reversed.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.orientation = Some(orientation);
        self.flip = self.kind == kind::SLIDER && (orientation == 90 || orientation == 180);
    }

    pub fn value_changed(&self, vindex: usize) -> bool {
        self.previous_values[vindex] != self.values[vindex]
    }

    /// timing-derived press events, additive to the `pressed` / `released` edges:
    /// a long press (held past [`LONG_PRESS`], reported on release) and double /
    /// triple presses (repeats within [`MULTI_TAP`]). button / dial only.
    pub fn press_events(&mut self, now: Instant) -> Vec<&'static str> {
        let mut types = Vec::new();
        if !(self.value_changed(0) && matches!(self.kind, kind::BUTTON | kind::DIAL)) {
            return types;
        }
        if self.values[0] == 1 {
            // press edge: count taps inside the multi-tap window, mark the hold.
            self.press_streak = match self.last_press {
                Some(prev) if now.duration_since(prev) < MULTI_TAP => self.press_streak + 1,
                _ => 1,
            };
            self.last_press = Some(now);
            self.press_started = Some(now);
            match self.press_streak {
                2 => types.push(input::DOUBLE_PRESSED),
                3 => {
                    types.push(input::TRIPLE_PRESSED);
                    self.press_streak = 0;
                }
                _ => {}
            }
        } else if self.values[0] == 0 {
            // release edge: a long enough hold is a long press.
            if let Some(started) = self.press_started.take() {
                if now.duration_since(started) >= LONG_PRESS {
                    types.push(input::LONG_PRESSED);
                }
            }
        }
        types
    }

    /// the value register that carries the dominant signal for an input type.
    pub fn determine_vindex(&self, input_type: &str) -> usize {
        let (base, index) = split_index(input_type);
        if matches!(
            base,
            input::PRESSED
                | input::RELEASED
                | input::LONG_PRESSED
                | input::DOUBLE_PRESSED
                | input::TRIPLE_PRESSED
        ) {
            return 0; // the press bitmask lives in v[0] for every kind
        }
        match self.kind {
            kind::DIAL => match base {
                input::SET => 3,
                input::ROTATION_LEFT => 1,
                input::ROTATION_RIGHT => 2,
                _ => 0,
            },
            kind::TRI_SLIDER if base == input::SET => {
                index.filter(|n| (1..=3).contains(n)).unwrap_or(1)
            }
            // the orbiter reads its signed rotation delta from v[0], and its
            // joystick x / y from the low byte of each signed-16 axis register.
            kind::ORBITER => match base {
                input::JOYSTICK_X => 2,
                input::JOYSTICK_Y => 4,
                _ => 0,
            },
            // the tri-dial's knob index selects its register: v[1] / v[2] / v[3]
            // (the rotation delta, or the integrated `set` position).
            kind::TRI_DIAL
                if matches!(
                    base,
                    input::ROTATION_LEFT | input::ROTATION_RIGHT | input::SET
                ) =>
            {
                index.filter(|n| (1..=3).contains(n)).unwrap_or(1)
            }
            _ => 0,
        }
    }

    /// the raw value for an input event. orbiter and tri-dial rotation registers
    /// hold a signed-byte delta (255 == -1), so they are sign-extended here.
    pub fn dominant_value(&self, input_type: &str) -> i64 {
        let (base, _) = split_index(input_type);
        // the orbiter joystick axes are little-endian signed-16 across two
        // registers (x = v[2..3], y = v[4..5]), not a single signed byte.
        if self.kind == kind::ORBITER {
            match base {
                input::JOYSTICK_X => return signed_axis(self.values[1], self.values[2]),
                input::JOYSTICK_Y => return signed_axis(self.values[3], self.values[4]),
                _ => {}
            }
        }
        // the orbiter / tri-dial `set` is the host-integrated absolute position.
        if base == input::SET && matches!(self.kind, kind::ORBITER | kind::TRI_DIAL) {
            return self.accumulated[self.determine_vindex(input_type)];
        }
        let raw = self.values[self.determine_vindex(input_type)];
        if matches!(self.kind, kind::ORBITER | kind::TRI_DIAL)
            && matches!(base, input::ROTATION_LEFT | input::ROTATION_RIGHT)
        {
            signed_delta(raw)
        } else {
            raw
        }
    }

    /// integrate this frame's rotation deltas into the persistent 0-255 dial
    /// position of each turning register, clamped to 0-255 so it tracks like a
    /// palettegear dial's `set` (full clockwise holds at 255, not wrapping to 0).
    /// the orbiter wheel (v[0]) and the tri-dial knobs (v[1..3]) are delta-only,
    /// so their absolute `set` position is synthesized here as input is processed
    /// (polling the raw delta would miss or double-count events).
    pub fn accumulate(&mut self) {
        let registers: &[usize] = match self.kind {
            kind::ORBITER => &[0],
            kind::TRI_DIAL => &[1, 2, 3],
            _ => return,
        };
        self.previous_accumulated = self.accumulated;
        for &i in registers {
            let delta = signed_delta(self.values[i]);
            self.accumulated[i] =
                (self.accumulated[i] + delta * ROTATION_SENSITIVITY).clamp(0, 255);
        }
    }

    /// the input events implied by the current versus previous value registers.
    /// the monogram modules (core, tri-dial, orbiter) were learned from hardware
    /// (see docs/monogram-protocol.md); their multi-control parts emit indexed
    /// input types (`pressed_2`, `rotation_left_3`, ...) so each sub-control binds
    /// independently.
    pub fn determine_input_types(&self) -> Vec<&'static str> {
        let mut types: Vec<&'static str> = Vec::new();
        let add = |types: &mut Vec<&'static str>, t: &'static str| {
            if !types.contains(&t) {
                types.push(t);
            }
        };

        // square button / dial press and release: v[0] toggles 1 (down) / 0 (up).
        if self.value_changed(0) && matches!(self.kind, kind::BUTTON | kind::DIAL) {
            if self.values[0] == 1 {
                add(&mut types, input::PRESSED);
            }
            if self.values[0] == 0 {
                add(&mut types, input::RELEASED);
            }
        }

        // core (2 buttons) / tri-dial (3 dial presses): v[0] is a press bitmask,
        // one bit per sub-button. per-bit edges become indexed press / release.
        if self.value_changed(0)
            && matches!(self.kind, kind::CORE | kind::TRI_DIAL | kind::TRI_BUTTON)
        {
            for n in [1usize, 2, 3] {
                let bit = 1i64 << (n - 1);
                let down = self.values[0] & bit != 0;
                let was_down = self.previous_values[0] & bit != 0;
                if down && !was_down {
                    add(&mut types, pressed_n(n));
                }
                if !down && was_down {
                    add(&mut types, released_n(n));
                }
            }
        }

        // square dial: rotation magnitude on v[1] (left) / v[2] (right), set on v[3].
        if self.kind == kind::DIAL {
            // re-assert the set position on any turn (like the orbiter wheel), so
            // spinning past 0/255 -- where the device stops changing v[3] -- keeps
            // displaying the clamped limit value rather than going stale.
            if self.value_changed(3) || self.values[1] > 0 || self.values[2] > 0 {
                add(&mut types, input::SET);
            }
            if self.values[2] > 0 {
                add(&mut types, input::ROTATION_RIGHT);
            }
            if self.values[1] > 0 {
                add(&mut types, input::ROTATION_LEFT);
            }
        }

        // orbiter: a signed rotation delta in v[0] (positive = right), plus a
        // joystick whose x / y stream as big-endian signed-16 in v[1..2] /
        // v[3..4] (a nonzero axis means it moved this frame).
        if self.kind == kind::ORBITER {
            add_rotation(&mut types, &add, self.values[0]);
            if signed_delta(self.values[0]) != 0 {
                add(&mut types, input::SET); // the integrated wheel position
            }
            if signed_axis(self.values[1], self.values[2]) != 0 {
                add(&mut types, input::JOYSTICK_X);
            }
            if signed_axis(self.values[3], self.values[4]) != 0 {
                add(&mut types, input::JOYSTICK_Y);
            }
        }

        // tri-dial: three knobs, each a signed delta on v[1] / v[2] / v[3], each
        // emitting its own indexed rotation (so the knobs bind separately) plus an
        // indexed `set` carrying the integrated absolute position of that knob.
        if self.kind == kind::TRI_DIAL {
            for n in [1usize, 2, 3] {
                match signed_delta(self.values[n]) {
                    delta if delta > 0 => add(&mut types, rotation_right_n(n)),
                    delta if delta < 0 => add(&mut types, rotation_left_n(n)),
                    _ => continue,
                }
                add(&mut types, set_n(n));
            }
        }

        // Official slider3x actuation maps v[1..3] to three absolute sliders.
        // Emit only the changed channel, including a return to zero.
        if self.kind == kind::TRI_SLIDER {
            for n in 1..=3 {
                if self.value_changed(n) {
                    add(&mut types, set_n(n));
                }
            }
        }

        if self.kind == kind::SLIDER {
            add(&mut types, input::SET);
        }

        types
    }

    /// a one-line decode of the latest input event, for the `inspect` command:
    /// the raw registers, which changed since the previous event, their signed-8
    /// reading, and the input types the engine derives. the trailing label is
    /// the decoded types, or `(neutral)` for the all-zero frame the device sends
    /// to reset a delta after a move, or `(unrecognized)` for a nonzero input
    /// the decoder does not model yet -- the signal that a control needs support.
    pub fn inspect_line(&self) -> String {
        let changed: Vec<String> = (0..8)
            .filter(|&i| self.values[i] != self.previous_values[i])
            .map(|i| format!("v{i}"))
            .collect();
        let decoded: Vec<String> = self
            .determine_input_types()
            .into_iter()
            .map(|t| format!("{t}={}", self.dominant_value(t)))
            .collect();
        let label = if decoded.is_empty() {
            crate::inspect::undecoded_label(&self.values).to_string()
        } else {
            decoded.join(" ")
        };
        format!(
            "[in] i={} {} raw=[{}] changed={} signed8=[{}] => {}",
            self.index,
            self.kind_name(),
            join_i64(&self.values),
            if changed.is_empty() {
                "none".to_string()
            } else {
                changed.join(",")
            },
            join_i64(&self.values.map(signed_delta)),
            label,
        )
    }
}

/// join eight value registers as a comma-separated list for inspect output.
fn join_i64(values: &[i64; 8]) -> String {
    values
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// the indexed `pressed_n` input type for sub-button `n`.
fn pressed_n(n: usize) -> &'static str {
    match n {
        1 => input::PRESSED_1,
        2 => input::PRESSED_2,
        3 => input::PRESSED_3,
        _ => input::PRESSED,
    }
}

fn released_n(n: usize) -> &'static str {
    match n {
        1 => input::RELEASED_1,
        2 => input::RELEASED_2,
        3 => input::RELEASED_3,
        _ => input::RELEASED,
    }
}

fn rotation_left_n(n: usize) -> &'static str {
    match n {
        1 => input::ROTATION_LEFT_1,
        2 => input::ROTATION_LEFT_2,
        3 => input::ROTATION_LEFT_3,
        _ => input::ROTATION_LEFT,
    }
}

/// the indexed `set_n` (integrated absolute position) for tri-dial knob `n`.
fn set_n(n: usize) -> &'static str {
    match n {
        1 => input::SET_1,
        2 => input::SET_2,
        3 => input::SET_3,
        _ => input::SET,
    }
}

fn rotation_right_n(n: usize) -> &'static str {
    match n {
        1 => input::ROTATION_RIGHT_1,
        2 => input::ROTATION_RIGHT_2,
        3 => input::ROTATION_RIGHT_3,
        _ => input::ROTATION_RIGHT,
    }
}

/// split an input type into its base and an optional `_<n>` sub-control index,
/// e.g. `"rotation_left_2"` -> `("rotation_left", Some(2))`, `"pressed"` ->
/// `("pressed", None)`. multi-control modules (tri-dial knobs, core buttons) use
/// the index to address one sub-control while the engine reads the base type.
pub fn split_index(input_type: &str) -> (&str, Option<usize>) {
    if let Some((base, suffix)) = input_type.rsplit_once('_') {
        if let Ok(index) = suffix.parse::<usize>() {
            return (base, Some(index));
        }
    }
    (input_type, None)
}

/// sign-extend a value register read as a signed 8-bit rotation delta, the
/// encoding the orbiter and tri-dial use (`255` == `-1`, `254` == `-2`, ...).
pub fn signed_delta(value: i64) -> i64 {
    if (128..256).contains(&value) {
        value - 256
    } else {
        value
    }
}

/// decode a big-endian signed 16-bit axis from a high / low register pair, the
/// encoding the orbiter joystick uses for each of its x / y positions (the high
/// byte sits at the lower register index).
pub fn signed_axis(high: i64, low: i64) -> i64 {
    let raw = ((high & 0xff) << 8) | (low & 0xff);
    if raw >= 0x8000 {
        raw - 0x10000
    } else {
        raw
    }
}

/// add the rotation direction implied by a signed delta register.
fn add_rotation(
    types: &mut Vec<&'static str>,
    add: &impl Fn(&mut Vec<&'static str>, &'static str),
    raw: i64,
) {
    match signed_delta(raw) {
        delta if delta > 0 => add(types, input::ROTATION_RIGHT),
        delta if delta < 0 => add(types, input::ROTATION_LEFT),
        _ => {}
    }
}

/// the connected component graph for one device layout.
#[derive(Debug, Default)]
pub struct ComponentLayout {
    pub base_orientation: Orientation,
    pub components: HashMap<String, Component>,
    /// the most recent layout tree, retained so a rotation can re-traverse it.
    pub layout_dict: Value,
    /// the index of the root component (the base / core that carries the screen).
    /// palettegear numbers it 1; monogram numbers it 0.
    base_index: i64,
    components_index: BTreeMap<i64, String>,
    components_slot: BTreeMap<Slot, String>,
}

/// the parent context threaded through a traversal: enough geometry to place a
/// child without borrowing back into the components map.
struct Parent {
    uid: String,
    slot: Slot,
    orientation: Orientation,
    num_ports: usize,
}

impl ComponentLayout {
    pub fn new(base_orientation: Orientation) -> ComponentLayout {
        ComponentLayout {
            base_orientation,
            ..Default::default()
        }
    }

    /// the number of currently connected components. the `components` map itself
    /// retains disconnected entries by uid so their stored state survives a
    /// reconnect, so the active count comes from the rebuilt index.
    pub fn len(&self) -> usize {
        self.components_index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.components_index.is_empty()
    }

    /// rebuild the index and slot maps from a fresh layout tree, reusing any
    /// existing component objects (and their stored state) by uid.
    pub fn update(&mut self, layout: &Value) {
        self.components_index.clear();
        self.components_slot.clear();
        self.traverse(layout, None, None);
        if self.find_component(self.base_index).map(|c| c.kind) == Some(kind::CORE) {
            self.apply_monogram_geometry();
        }
        self.layout_dict = layout.clone();
    }

    /// the monogram reports a linear connection chain, but the modules are
    /// physically a 2d multi-cell arrangement: the core (2x1, screen left /
    /// buttons right) with the orbiter (2x2) below-left and the tri-dial (1x2)
    /// below the buttons. place each module by rotating its known cell set by the
    /// base orientation (so the layout turns with the rest of the setup), baking
    /// the result into the anchor slot and footprint; the slot index covers every
    /// occupied cell so led-draw and `component_in_slot` match the hardware.
    fn apply_monogram_geometry(&mut self) {
        let orientation = self.base_orientation;
        for component in self.components.values_mut() {
            let cells = monogram_cells(component.kind);
            if cells.is_empty() {
                continue;
            }
            let rotated: Vec<Slot> = cells.iter().map(|&c| rotate_slot(c, orientation)).collect();
            let min_x = rotated.iter().map(|c| c.0).min().unwrap();
            let max_x = rotated.iter().map(|c| c.0).max().unwrap();
            let min_y = rotated.iter().map(|c| c.1).min().unwrap();
            let max_y = rotated.iter().map(|c| c.1).max().unwrap();
            component.slot = Some((min_x, max_y)); // top-left anchor
            component.footprint = (max_x - min_x + 1, max_y - min_y + 1);
            component.orientation = Some(0); // rotation is baked into slot + footprint
        }
        self.components_slot.clear();
        let placed: Vec<(Slot, (i32, i32), String)> = self
            .components
            .values()
            .filter(|c| !monogram_cells(c.kind).is_empty())
            .map(|c| (c.slot.unwrap_or((0, 0)), c.footprint, c.uid.clone()))
            .collect();
        for ((ax, ay), (fw, fh), uid) in placed {
            for dx in 0..fw {
                for dy in 0..fh {
                    self.components_slot.insert((ax + dx, ay - dy), uid.clone());
                }
            }
        }
    }

    fn traverse(
        &mut self,
        node: &Value,
        parent: Option<&Parent>,
        port: Option<usize>,
    ) -> Option<String> {
        if node.is_null() {
            return None;
        }
        let uid = node.get("u")?.as_str()?.trim_start().to_string();
        let kind = node.get("t").and_then(Value::as_u64).unwrap_or(0) as u8;
        let index = node.get("i").and_then(Value::as_i64).unwrap_or(0);
        let conns = node
            .get("c")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if parent.is_none() {
            self.base_index = index;
        }

        let orientation = self.calculate_orientation(parent, port);
        let slot = calculate_slot(parent, port);
        let num_ports = conns.len();

        let mut component = self
            .components
            .remove(&uid)
            .unwrap_or_else(|| Component::new(index, &uid, kind));
        component.index = index;
        component.kind = kind;
        component.set_orientation(orientation);
        component.num_ports = Some(num_ports);
        component.slot = Some(slot);
        component.parent_uid = parent.map(|p| p.uid.clone());
        component.parent_port = port;
        component.children = HashMap::new();

        let child_parent = Parent {
            uid: uid.clone(),
            slot,
            orientation,
            num_ports,
        };
        for (p, child) in conns.iter().enumerate() {
            if let Some(child_uid) = self.traverse(child, Some(&child_parent), Some(p)) {
                component.children.insert(p, child_uid);
            }
        }

        self.components.insert(uid.clone(), component);
        self.components_index.insert(index, uid.clone());
        self.components_slot.insert(slot, uid.clone());
        Some(uid)
    }

    fn calculate_orientation(&self, parent: Option<&Parent>, port: Option<usize>) -> Orientation {
        let Some(port) = port else {
            return self.base_orientation;
        };
        let parent = parent.expect("non-root node has a parent");
        // a wide (five-port) parent folds its port indices so that ports 2-3
        // share port 1's facing and port 0 rotates back an extra 90 degrees.
        let mut port = port as i64;
        if parent.num_ports == 5 {
            port = if port == 0 { -1 } else { port / 2 };
        }
        ((parent.orientation as i64 - 90) + port * 90).rem_euclid(360) as Orientation
    }

    /// components in attachment-index order.
    pub fn connected(&self) -> Vec<&Component> {
        self.components_index
            .values()
            .filter_map(|uid| self.components.get(uid))
            .collect()
    }

    /// components ordered top-to-bottom then left-to-right by slot.
    pub fn connected_rowwise(&self) -> Vec<&Component> {
        sorted_rowwise(self.connected())
    }

    pub fn find_component(&self, index: i64) -> Option<&Component> {
        self.components_index
            .get(&index)
            .and_then(|uid| self.components.get(uid))
    }

    pub fn find_component_mut(&mut self, index: i64) -> Option<&mut Component> {
        let uid = self.components_index.get(&index)?.clone();
        self.components.get_mut(&uid)
    }

    /// the index of the base / core component (the one with the screen).
    pub fn base_index(&self) -> i64 {
        self.base_index
    }

    /// the base / core component, where the screen string and icon live.
    pub fn base(&self) -> Option<&Component> {
        self.find_component(self.base_index)
    }

    pub fn base_mut(&mut self) -> Option<&mut Component> {
        self.find_component_mut(self.base_index)
    }

    pub fn component_in_slot(&self, slot: Slot) -> Option<&Component> {
        self.components_slot
            .get(&slot)
            .and_then(|uid| self.components.get(uid))
    }

    /// the bounding box of occupied slots, expanded for wide components.
    pub fn extrema(&self) -> (i32, i32, i32, i32) {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (0, 0, 0, 0);
        for component in self.connected() {
            let (ax, ay) = component.slot.unwrap_or((0, 0));
            let (mut x, mut y) = (ax, ay);
            // the wide palettegear slider extends one cell in its orientation
            // (gated to the slider kind: the tri-dial also reports 5 ports).
            if component.kind == kind::SLIDER && component.num_ports == Some(5) {
                match component.orientation {
                    Some(0) => x += 1,
                    Some(90) => y -= 1,
                    Some(180) => x -= 1,
                    Some(270) => y += 1,
                    _ => {}
                }
            }
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            // multi-cell monogram modules span right (+x) and down (-y) by their
            // (possibly rotated) footprint from the top-left anchor.
            if matches!(component.kind, kind::CORE | kind::TRI_DIAL | kind::ORBITER) {
                let (fw, fh) = component.footprint;
                max_x = max_x.max(ax + fw - 1);
                min_y = min_y.min(ay - (fh - 1));
            }
        }
        (min_x, min_y, max_x, max_y)
    }

    pub fn query<'a>(&'a self, query: &str) -> Vec<&'a Component> {
        let (spec, select) = parse_query(query);
        let mut components = self.search(&spec);
        if let Some(select) = select {
            if select == 0 || components.len() < select {
                return Vec::new();
            }
            components = vec![components[select - 1]];
        }
        components
    }

    fn search<'a>(&'a self, spec: &QuerySpec) -> Vec<&'a Component> {
        let mut components: Vec<&Component> = self
            .connected()
            .into_iter()
            .filter(|c| spec.matches(c))
            .collect();
        if spec.rowwise {
            components = sorted_rowwise(components);
        }
        components
    }
}

fn calculate_slot(parent: Option<&Parent>, port: Option<usize>) -> Slot {
    let (Some(parent), Some(port)) = (parent, port) else {
        return (0, 0);
    };
    let slots: Vec<Slot> = if parent.num_ports == 5 {
        calculate_female_slots_wide(parent.slot, parent.orientation).to_vec()
    } else {
        calculate_female_slots(parent.slot, parent.orientation).to_vec()
    };
    // Monogram's seven-port modules can report children beyond the legacy
    // three/five-port geometry tables. Keep those children in the graph;
    // known Monogram kinds receive their display placement after traversal.
    // An unmapped port has provisional parent placement, not guessed geometry.
    slots.get(port).copied().unwrap_or(parent.slot)
}

fn sorted_rowwise(mut components: Vec<&Component>) -> Vec<&Component> {
    components.sort_by_key(|c| {
        let (x, y) = c.slot.unwrap_or((0, 0));
        (-y, x)
    });
    components
}

/// a parsed layout query: a set of field matchers plus a rowwise toggle.
#[derive(Debug, Default)]
struct QuerySpec {
    uid: Option<String>,
    kind: Option<u8>,
    index: Option<i64>,
    slot: Option<Slot>,
    orientation: Option<Orientation>,
    rowwise: bool,
}

impl QuerySpec {
    fn matches(&self, c: &Component) -> bool {
        if let Some(uid) = &self.uid {
            if &c.uid != uid {
                return false;
            }
        }
        if let Some(kind) = self.kind {
            if c.kind != kind {
                return false;
            }
        }
        if let Some(slot) = self.slot {
            if c.slot != Some(slot) {
                return false;
            }
        }
        if let Some(index) = self.index {
            if c.index != index {
                return false;
            }
        }
        if let Some(orientation) = self.orientation {
            if c.orientation != Some(orientation) {
                return false;
            }
        }
        true
    }
}

fn parse_slot(value: &str) -> Option<Slot> {
    let inner = value.trim_start_matches('(').trim_end_matches(')');
    let (x, y) = inner.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

/// parse a query into its matcher spec and optional 1-based `select(n)` index.
fn parse_query(query: &str) -> (QuerySpec, Option<usize>) {
    if !(query.starts_with('[') && query.ends_with(']')) {
        return (
            QuerySpec {
                uid: Some(query.to_string()),
                ..Default::default()
            },
            None,
        );
    }

    let inner = query.trim_start_matches('[').trim_end_matches(']');
    let mut parts = inner.split(" | ");
    let matcher = parts.next().unwrap_or("");

    let mut spec = QuerySpec::default();
    for token in matcher.split(' ') {
        let Some((k, v)) = token.split_once('=') else {
            continue;
        };
        match k {
            "kind" => spec.kind = kind_from_name(v),
            "index" => spec.index = v.parse().ok(),
            "orientation" => spec.orientation = v.parse().ok(),
            "slot" => spec.slot = parse_slot(v),
            "uid" => spec.uid = Some(v.to_string()),
            _ => {}
        }
    }

    let mut select = None;
    for f in parts {
        if f == "rowwise" {
            spec.rowwise = true;
        } else if let Some(n) = f.strip_prefix("select(").and_then(|s| s.strip_suffix(')')) {
            select = n.parse().ok();
        }
    }

    (spec, select)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn expertkit() -> Value {
        // mirrors fake.py LAYOUT_DEFAULT_EXPERTKIT; uids carry a leading "  "
        // that traverse strips. used as the upstream traversal oracle.
        json!({"u":"00000","i":1,"t":0,"c":[
            null,
            {"u":"  49d","i":2,"t":2,"c":[
                {"u":"  4FH","i":3,"t":2,"c":[
                    {"u":"  4lf","i":8,"t":1,"c":[null,null,null]},
                    {"u":"  4}=","i":4,"t":3,"c":[null,null,null,null,null]},
                    null
                ]},
                null,
                {"u":"  4A0","i":5,"t":2,"c":[
                    null,
                    null,
                    {"u":"  4ls","i":6,"t":1,"c":[
                        {"u":"  4}M","i":7,"t":3,"c":[null,null,null,null,null]},
                        null,
                        null
                    ]}
                ]}
            ]},
            null
        ]})
    }

    fn built_expertkit() -> ComponentLayout {
        let mut layout = ComponentLayout::new(0);
        layout.update(&expertkit());
        layout
    }

    #[test]
    fn footprints_match_module_shapes() {
        assert_eq!((2, 1), footprint(kind::CORE));
        assert_eq!((1, 2), footprint(kind::TRI_DIAL));
        assert_eq!((2, 2), footprint(kind::ORBITER));
        assert_eq!((1, 1), footprint(kind::BUTTON));
    }

    #[test]
    fn monogram_geometry_places_modules_in_a_2d_grid() {
        // the device reports a linear chain (core -> tri-dial -> orbiter); the
        // layout re-places them in the physical 2d arrangement.
        let tree = json!({"u":"core","i":0,"t":kind::CORE,"c":[null,null,null,
            {"u":"tri","i":1,"t":kind::TRI_DIAL,"c":[null,null,null,
                {"u":"orb","i":2,"t":kind::ORBITER,"c":[null,null,null,null,null,null,null]},
                null]},
            null]});
        let mut layout = ComponentLayout::new(0);
        layout.update(&tree);
        // at orientation 0 the device is screen-upright: core vertical on the
        // right, orbiter to its upper-left, tri-dial below the orbiter.
        assert_eq!(Some((0, 0)), layout.find_component(0).unwrap().slot); // core
        assert_eq!(Some((-2, -1)), layout.find_component(1).unwrap().slot); // tri-dial
        assert_eq!(Some((-2, 1)), layout.find_component(2).unwrap().slot); // orbiter
                                                                           // the footprints span a 3x3 grid.
        assert_eq!((-2, -1, 0, 1), layout.extrema());
        // an orbiter footprint cell resolves to the orbiter.
        assert_eq!(Some(2), layout.component_in_slot((-1, 1)).map(|c| c.index));
    }

    #[test]
    fn monogram_geometry_rotates_with_the_base_orientation() {
        let tree = json!({"u":"core","i":0,"t":kind::CORE,"c":[null,null,null,
            {"u":"tri","i":1,"t":kind::TRI_DIAL,"c":[null,null,null,
                {"u":"orb","i":2,"t":kind::ORBITER,"c":[null,null,null,null,null,null,null]},
                null]},
            null]});
        // rotated 90 degrees clockwise from screen-upright, each module's
        // footprint swaps and it moves: the vertical core becomes horizontal,
        // the horizontal tri-dial becomes vertical.
        let mut layout = ComponentLayout::new(90);
        layout.update(&tree);
        let core = layout.find_component(0).unwrap();
        assert_eq!((Some((-1, 0)), (2, 1)), (core.slot, core.footprint));
        let tri = layout.find_component(1).unwrap();
        assert_eq!((Some((-1, 2)), (1, 2)), (tri.slot, tri.footprint));
        let orbiter = layout.find_component(2).unwrap();
        assert_eq!((Some((0, 2)), (2, 2)), (orbiter.slot, orbiter.footprint));
    }

    #[test]
    fn traverse_assigns_slots_and_orientations() {
        let layout = built_expertkit();
        assert_eq!(8, layout.connected().len());

        let orient = |i| layout.find_component(i).unwrap().orientation.unwrap();
        assert_eq!(0, orient(1));
        assert_eq!(0, orient(2));
        assert_eq!(270, orient(3));
        assert_eq!(270, orient(4));
        assert_eq!(90, orient(5));
        assert_eq!(180, orient(6));
        assert_eq!(90, orient(7));
        assert_eq!(180, orient(8));

        let slot = |i| layout.find_component(i).unwrap().slot.unwrap();
        assert_eq!((0, 0), slot(1));
        assert_eq!((0, -1), slot(2));
        assert_eq!((1, -1), slot(3));
        assert_eq!((-1, -1), slot(5));

        assert!(!layout.find_component(4).unwrap().flip);
        assert!(layout.find_component(7).unwrap().flip);
    }

    #[test]
    fn query_dsl_matches_upstream() {
        let layout = built_expertkit();
        let q = |s| layout.query(s);
        assert_eq!("49d", q("[kind=Dial | rowwise | select(2)]")[0].uid);
        assert_eq!("49d", q("49d")[0].uid);
        assert_eq!("4lf", q("[slot=(1,0)]")[0].uid);
        assert_eq!("4}=", q("[kind=Slider orientation=270]")[0].uid);
        assert_eq!("00000", q("[index=1]")[0].uid);
        assert_eq!("4}=", q("[uid=4}=]")[0].uid);
        assert_eq!(3, q("[kind=Dial]").len());
        assert_eq!(2, q("[orientation=90]").len());
    }

    #[test]
    fn cycle_behavior_advances_and_loops() {
        let mut behavior = Behavior::new(BehaviorKind::Cycle);
        behavior.set_items(
            ["ONE", "TWO", "THREE", "FOUR", "FIVE", "SIX"]
                .iter()
                .map(|s| json!(s))
                .collect(),
        );
        behavior.loops = Some(true);
        behavior.nested = Some(true);
        let mut inv = Invocation::new(input::RELEASED, "testCycle", behavior, 0, vec![]);

        let mut stored = 0.0;
        // first release advances from the default index 0 to item "TWO".
        assert_eq!(
            Some(DispatchValue::Item {
                index: 1,
                item: json!("TWO")
            }),
            inv.run(0, 1, &mut stored)
        );
        // advance to the last item, then wrap back to the first.
        stored = 5.0;
        assert_eq!(
            Some(DispatchValue::Item {
                index: 0,
                item: json!("ONE")
            }),
            inv.run(0, 1, &mut stored)
        );
    }

    #[test]
    fn accumulate_respects_threshold_gate() {
        let mut behavior = Behavior::new(BehaviorKind::Accumulate);
        behavior.threshold = Some(16);
        let mut inv = Invocation::new(input::ROTATION_LEFT, "testAccumulate", behavior, 1, vec![]);

        let mut stored = 0.0;
        // a rotation below the threshold does not fire.
        assert_eq!(None, inv.run(8, 0, &mut stored));
        // crossing the threshold fires once and accumulates by the threshold.
        assert_eq!(
            Some(DispatchValue::Number(16.0)),
            inv.run(8, 0, &mut stored)
        );
        assert_eq!(16.0, stored);
    }

    #[test]
    fn trigger_on_limit_is_mandatory() {
        let mut behavior = Behavior::new(BehaviorKind::Trigger);
        behavior.limits = Some((0, 255));
        behavior.threshold = Some(1000);
        let mut inv = Invocation::new(input::SET, "testTrigger", behavior, 0, vec![]);

        // value 255 lands on a limit, so it fires despite the high threshold.
        let mut stored = 0.0;
        assert_eq!(
            Some(DispatchValue::Number(255.0)),
            inv.run(255, 0, &mut stored)
        );
    }

    // -- monogram modules, from real-hardware captures (docs/monogram-protocol.md) --

    fn component_with(component_kind: u8, previous: [i64; 8], values: [i64; 8]) -> Component {
        let mut component = Component::new(1, "x", component_kind);
        component.previous_values = previous;
        component.values = values;
        component
    }

    #[test]
    fn monogram_core_buttons_emit_indexed_presses() {
        // captured v[0] bitmask: bit 1 = button a, bit 2 = button b.
        let button_a = component_with(kind::CORE, [0; 8], [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(vec![input::PRESSED_1], button_a.determine_input_types());

        let both = component_with(kind::CORE, [0; 8], [3, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::PRESSED_1, input::PRESSED_2],
            both.determine_input_types()
        );

        // releasing button b (bit 2 goes 1 -> 0) fires released_2.
        let release_b = component_with(kind::CORE, [2, 0, 0, 0, 0, 0, 0, 0], [0; 8]);
        assert_eq!(vec![input::RELEASED_2], release_b.determine_input_types());
    }

    #[test]
    fn dial_reasserts_set_while_rotating_at_the_limit() {
        // at the 0/255 limit the device stops changing v[3]; the dial must still
        // emit `set` while rotating so the clamped position keeps displaying.
        let dial = component_with(
            kind::DIAL,
            [0, 0, 0, 255, 0, 0, 0, 0],
            [0, 0, 1, 255, 0, 0, 0, 0],
        );
        let types = dial.determine_input_types();
        assert!(types.contains(&input::SET), "set re-asserted: {types:?}");
        assert!(types.contains(&input::ROTATION_RIGHT));
        assert_eq!(255, dial.dominant_value(input::SET));
    }

    #[test]
    fn monogram_orbiter_rotation_is_a_signed_v0_delta() {
        // captured: v[0] = 1 (right), 255 = -1 (left), 254 = -2.
        // each rotation also emits the integrated `set` position.
        let right = component_with(kind::ORBITER, [0; 8], [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::ROTATION_RIGHT, input::SET],
            right.determine_input_types()
        );
        assert_eq!(1, right.dominant_value(input::ROTATION_RIGHT));

        let left = component_with(kind::ORBITER, [0; 8], [255, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::ROTATION_LEFT, input::SET],
            left.determine_input_types()
        );
        assert_eq!(-1, left.dominant_value(input::ROTATION_LEFT));

        let left2 = component_with(kind::ORBITER, [0; 8], [254, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(-2, left2.dominant_value(input::ROTATION_LEFT));
    }

    #[test]
    fn monogram_orbiter_joystick_is_two_signed_16_axes() {
        // captured x/y are big-endian signed-16: x = v[1..2], y = v[3..4].
        // [0,255,254,0,2,0,0,0] -> x = 0xfffe = -2, y = 0x0002 = +2.
        let diag = component_with(kind::ORBITER, [0; 8], [0, 255, 254, 0, 2, 0, 0, 0]);
        assert_eq!(
            vec![input::JOYSTICK_X, input::JOYSTICK_Y],
            diag.determine_input_types()
        );
        assert_eq!(-2, diag.dominant_value(input::JOYSTICK_X));
        assert_eq!(2, diag.dominant_value(input::JOYSTICK_Y));

        // a one-axis push: x only (y bytes zero) -> just joystick_x.
        let x_only = component_with(kind::ORBITER, [0; 8], [0, 255, 250, 0, 0, 0, 0, 0]);
        assert_eq!(vec![input::JOYSTICK_X], x_only.determine_input_types());
        assert_eq!(-6, x_only.dominant_value(input::JOYSTICK_X));

        // a full-deflection capture where y exceeds the signed-8 range (253),
        // proving the axis is 16-bit rather than a single signed byte.
        let push = component_with(kind::ORBITER, [0; 8], [0, 0, 91, 0, 253, 43, 119, 0]);
        assert_eq!(91, push.dominant_value(input::JOYSTICK_X));
        assert_eq!(253, push.dominant_value(input::JOYSTICK_Y));

        // rotation and joystick are independent: a swirl carries both at once.
        let swirl = component_with(
            kind::ORBITER,
            [0; 8],
            [255, 255, 219, 255, 243, 248, 254, 0],
        );
        assert_eq!(
            vec![
                input::ROTATION_LEFT,
                input::SET,
                input::JOYSTICK_X,
                input::JOYSTICK_Y
            ],
            swirl.determine_input_types()
        );
        assert_eq!(-37, swirl.dominant_value(input::JOYSTICK_X));
        assert_eq!(-13, swirl.dominant_value(input::JOYSTICK_Y));
    }

    #[test]
    fn monogram_tridial_press_mask_and_per_knob_rotation() {
        // captured press bitmask: bit 1/2/4 (1 << (n-1)) for dials 1/2/3.
        let dial3 = component_with(kind::TRI_DIAL, [0; 8], [4, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(vec![input::PRESSED_3], dial3.determine_input_types());

        // knob on v[2] turning right -> rotation_right_2 + its set_2 position.
        let knob2 = component_with(kind::TRI_DIAL, [0; 8], [0, 0, 1, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::ROTATION_RIGHT_2, input::SET_2],
            knob2.determine_input_types()
        );
        assert_eq!(2, knob2.determine_vindex(input::ROTATION_RIGHT_2));
        assert_eq!(1, knob2.dominant_value(input::ROTATION_RIGHT_2));

        // knob on v[1] turning left (255 == -1) -> rotation_left_1 + set_1.
        let knob1 = component_with(kind::TRI_DIAL, [0; 8], [0, 255, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::ROTATION_LEFT_1, input::SET_1],
            knob1.determine_input_types()
        );
        assert_eq!(-1, knob1.dominant_value(input::ROTATION_LEFT_1));
    }

    #[test]
    fn inspect_line_decodes_known_input_and_flags_unknown() {
        // a recognized orbiter swirl: raw, changed registers, signed-8, decode.
        let mut swirl = component_with(
            kind::ORBITER,
            [0; 8],
            [255, 255, 219, 255, 243, 248, 254, 0],
        );
        swirl.index = 2;
        assert_eq!(
            "[in] i=2 Orbiter raw=[255,255,219,255,243,248,254,0] \
             changed=v0,v1,v2,v3,v4,v5,v6 signed8=[-1,-1,-37,-1,-13,-8,-2,0] \
             => rotation_left=-1 set=0 joystick_x=-37 joystick_y=-13",
            swirl.inspect_line()
        );

        // an unmodeled control (tri-dial v[0] = bit 4, past the three dials)
        // decodes to nothing and is flagged for follow-up.
        let mut unknown = component_with(kind::TRI_DIAL, [0; 8], [8, 0, 0, 0, 0, 0, 0, 0]);
        unknown.index = 1;
        assert_eq!(
            "[in] i=1 Tri-Dial raw=[8,0,0,0,0,0,0,0] changed=v0 \
             signed8=[8,0,0,0,0,0,0,0] => (unrecognized)",
            unknown.inspect_line()
        );
    }

    #[test]
    fn rotation_accumulates_into_a_clamped_dial_position() {
        // the orbiter wheel (v[0]) integrates into accumulated[0] at the
        // sensitivity, clamped to 0-255 like a palettegear dial -- the ends hold
        // rather than wrapping.
        let mut orbiter = component_with(kind::ORBITER, [0; 8], [1, 0, 0, 0, 0, 0, 0, 0]);
        orbiter.accumulate();
        assert_eq!(4, orbiter.accumulated[0]);
        orbiter.values = [255, 0, 0, 0, 0, 0, 0, 0]; // -1 each step
        orbiter.accumulate(); // 4 - 4 -> 0
        orbiter.accumulate(); // at the floor: holds 0, not wrapping to 252
        assert_eq!(0, orbiter.accumulated[0]);
        // and the ceiling holds at 255 rather than wrapping to 0.
        orbiter.accumulated[0] = 254;
        orbiter.values = [1, 0, 0, 0, 0, 0, 0, 0]; // +4
        orbiter.accumulate();
        assert_eq!(255, orbiter.accumulated[0]);

        // each tri-dial knob integrates its own register, recording the prior
        // position so the `set` behavior can compute its change.
        let mut tri = component_with(kind::TRI_DIAL, [0; 8], [0, 0, 1, 0, 0, 0, 0, 0]);
        tri.accumulate();
        assert_eq!(4, tri.accumulated[2]);
        assert_eq!(0, tri.accumulated[1]);
        tri.accumulate();
        assert_eq!((8, 4), (tri.accumulated[2], tri.previous_accumulated[2]));
    }

    #[test]
    fn orbiter_port_four_preserves_attached_tri_dial() {
        // Exact Core -> Orbiter -> Tri-Dial layout from the user's crash log.
        let tree = serde_json::json!({"u":"?uoEf","i":0,"t":25,"c":[null,null,null,
            {"u":"?x-|q","i":1,"t":24,"c":[null,null,null,null,
                {"u":"?wI-0","i":2,"t":9,"c":[null,null,null,null,null]},null,null]},null]});
        for orientation in [0, 90, 180, 270] {
            let mut layout = ComponentLayout::new(orientation);
            layout.update(&tree);
            assert_eq!(3, layout.len());
            let dial = layout.find_component(2).unwrap();
            assert_eq!(kind::TRI_DIAL, dial.kind);
            assert_eq!(Some(4), dial.parent_port);
            assert_eq!(Some("?x-|q"), dial.parent_uid.as_deref());
        }
    }

    #[test]
    fn seven_port_parent_traverses_every_attachment_port() {
        for port in 0..7 {
            let mut children = vec![Value::Null; 7];
            children[port] = serde_json::json!({"u":"dial","i":2,"t":9,"c":[]});
            let tree = serde_json::json!({"u":"core","i":0,"t":25,"c":[
                {"u":"orbiter","i":1,"t":24,"c":children},null,null,null,null]});
            let mut layout = ComponentLayout::new(0);
            layout.update(&tree);
            assert_eq!(Some(port), layout.find_component(2).unwrap().parent_port);
        }
    }

    #[test]
    fn tri_button_press_release_and_simultaneous_edges() {
        for n in 1..=3 {
            let mut c = component_with(kind::TRI_BUTTON, [0; 8], [0; 8]);
            c.values[0] = 1 << (n - 1);
            assert_eq!(vec![pressed_n(n)], c.determine_input_types());
            c.previous_values = c.values;
            c.values[0] = 0;
            assert_eq!(vec![released_n(n)], c.determine_input_types());
        }
        let c = component_with(
            kind::TRI_BUTTON,
            [1, 0, 0, 0, 0, 0, 0, 0],
            [6, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(
            vec![input::RELEASED_1, input::PRESSED_2, input::PRESSED_3],
            c.determine_input_types()
        );
        assert_eq!(Some(kind::TRI_BUTTON), kind_from_name(c.kind_name()));
    }

    #[test]
    fn tri_slider_decodes_independent_absolute_channels() {
        // Recorded kind-10 movement: v1 changes while the other faders stay put.
        let tri = component_with(
            kind::TRI_SLIDER,
            [0, 253, 255, 255, 0, 0, 0, 0],
            [0, 255, 255, 255, 0, 0, 0, 0],
        );
        assert_eq!(vec![input::SET_1], tri.determine_input_types());
        assert_eq!(255, tri.dominant_value(input::SET_1));
        assert_eq!("Tri-Slider", tri.kind_name());
        assert_eq!(Some(kind::TRI_SLIDER), kind_from_name("Tri-Slider"));
        for n in 1..=3 {
            let mut tri = component_with(kind::TRI_SLIDER, [0; 8], [0; 8]);
            tri.previous_values[n] = 255;
            assert_eq!(vec![set_n(n)], tri.determine_input_types());
            assert_eq!(n, tri.determine_vindex(set_n(n)));
            assert_eq!(0, tri.dominant_value(set_n(n)));
        }
        let tri = component_with(kind::TRI_SLIDER, [0; 8], [0; 8]);
        assert!(tri.determine_input_types().is_empty());
    }

    #[test]
    fn tri_dial_knob_exposes_an_integrated_set_position() {
        // a knob turn emits both its indexed rotation and an indexed `set`
        // carrying the integrated position; `set` reads the accumulator.
        let mut tri = component_with(kind::TRI_DIAL, [0; 8], [0, 0, 1, 0, 0, 0, 0, 0]);
        tri.accumulate();
        let types = tri.determine_input_types();
        assert!(types.contains(&input::ROTATION_RIGHT_2));
        assert!(types.contains(&input::SET_2));
        assert_eq!(2, tri.determine_vindex(input::SET_2));
        assert_eq!(4, tri.dominant_value(input::SET_2)); // accumulated[2]
    }

    #[test]
    fn inspect_line_flags_an_all_zero_reset_as_neutral() {
        // after a delta event the device sends an all-zero frame to reset the
        // registers; it decodes to nothing but is a neutral reset, not an
        // unrecognized control (observed live: the tri-dial / orbiter emit one
        // after every rotation or joystick move).
        let mut reset = component_with(kind::TRI_DIAL, [0, 255, 0, 0, 0, 0, 0, 0], [0; 8]);
        reset.index = 1;
        assert_eq!(
            "[in] i=1 Tri-Dial raw=[0,0,0,0,0,0,0,0] changed=v1 \
             signed8=[0,0,0,0,0,0,0,0] => (neutral)",
            reset.inspect_line()
        );
    }

    #[test]
    fn indexed_input_types_split_and_run_as_their_base() {
        assert_eq!(("rotation_left", Some(2)), split_index("rotation_left_2"));
        assert_eq!(("pressed", Some(1)), split_index("pressed_1"));
        assert_eq!(("rotation_left", None), split_index("rotation_left"));
        assert_eq!(("set", None), split_index("set"));

        // an indexed knob rotation accumulates exactly like the base type.
        let mut behavior = Behavior::new(BehaviorKind::Accumulate);
        behavior.threshold = Some(1);
        let mut inv = Invocation::new(
            input::ROTATION_LEFT_3,
            "testAccumulate",
            behavior,
            3,
            vec![],
        );
        let mut stored = 0.0;
        assert_eq!(Some(DispatchValue::Number(1.0)), inv.run(1, 0, &mut stored));
    }

    #[test]
    fn multi_press_events_fire_like_a_press() {
        // long / double / triple press are press-family, so a trigger bound to one
        // fires the same as `pressed` (long press reports on release, value 0).
        for input_type in [
            input::LONG_PRESSED,
            input::DOUBLE_PRESSED,
            input::TRIPLE_PRESSED,
        ] {
            let mut behavior = Behavior::new(BehaviorKind::Trigger);
            behavior.threshold = Some(1);
            let mut inv = Invocation::new(input_type, "previousFunction", behavior, 0, vec![]);
            let mut stored = 0.0;
            assert!(inv.run(0, 1, &mut stored).is_some(), "{input_type}");
        }
    }

    #[test]
    fn square_button_and_dial_input_unchanged() {
        // regression: the palettegear square modules must keep their behavior.
        let pressed = component_with(kind::BUTTON, [0; 8], [1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(vec![input::PRESSED], pressed.determine_input_types());

        // a turn re-asserts the set position (here v[3]=0) alongside the rotation.
        let dial_left = component_with(kind::DIAL, [0; 8], [0, 8, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            vec![input::SET, input::ROTATION_LEFT],
            dial_left.determine_input_types()
        );
        assert_eq!(8, dial_left.dominant_value(input::ROTATION_LEFT));
    }

    #[test]
    fn base_index_follows_the_root_component() {
        // palettegear numbers the base (root) component 1.
        let mut palette = ComponentLayout::new(0);
        palette.update(&expertkit());
        assert_eq!(1, palette.base_index());

        // monogram numbers the core (root, with the screen) 0.
        let mut monogram = ComponentLayout::new(0);
        monogram.update(&json!({"u":"core","i":0,"t":kind::CORE,"c":[
            {"u":"knob","i":1,"t":kind::TRI_DIAL,"c":[null,null,null]},
            null,
            null
        ]}));
        assert_eq!(0, monogram.base_index());
        assert_eq!(kind::CORE, monogram.base().unwrap().kind);
    }

    #[test]
    fn press_events_detect_long_double_and_triple() {
        let t0 = Instant::now();
        let press = |c: &mut Component, at: Instant| {
            c.previous_values[0] = 0;
            c.values[0] = 1;
            c.press_events(at)
        };
        let release = |c: &mut Component, at: Instant| {
            c.previous_values[0] = 1;
            c.values[0] = 0;
            c.press_events(at)
        };

        // rapid taps within the multi-tap window escalate to double then triple.
        let mut button = Component::new(1, "b", kind::BUTTON);
        assert!(press(&mut button, t0).is_empty()); // first tap: only `pressed`
        assert!(release(&mut button, t0 + Duration::from_millis(40)).is_empty());
        assert_eq!(
            vec![input::DOUBLE_PRESSED],
            press(&mut button, t0 + Duration::from_millis(180))
        );
        assert!(release(&mut button, t0 + Duration::from_millis(220)).is_empty());
        assert_eq!(
            vec![input::TRIPLE_PRESSED],
            press(&mut button, t0 + Duration::from_millis(360))
        );

        // a hold past the threshold reports a long press on release.
        let mut held = Component::new(2, "h", kind::BUTTON);
        assert!(press(&mut held, t0).is_empty());
        assert_eq!(
            vec![input::LONG_PRESSED],
            release(&mut held, t0 + Duration::from_millis(600))
        );
    }
}
