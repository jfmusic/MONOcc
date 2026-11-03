//! protocol analysis for the `inspect` command.
//!
//! [`InspectAnalysis`] is fed decoded input events (from the live controller or,
//! later, an offline pcap replay) and deduplicates them into a summary: per
//! (component, input type) it keeps an event count and the observed value range,
//! and per component it tracks which raw value registers carried signal and over
//! what range. that register view is what surfaces an undecoded channel -- e.g.
//! the orbiter joystick streams a third axis in `v[5..7]` that no input type
//! reads yet, but it shows up here as active registers.

use crate::component::ComponentLayout;
use serde_json::Value;
use std::collections::BTreeMap;

/// the trailing label for an input event that decoded to no input type: a delta
/// control sends an all-zero frame to reset after a move (`(neutral)`), versus a
/// nonzero frame nothing models yet (`(unrecognized)`, the signal to extend a
/// component's decode).
pub fn undecoded_label(values: &[i64; 8]) -> &'static str {
    if values.iter().all(|&v| v == 0) {
        "(neutral)"
    } else {
        "(unrecognized)"
    }
}

/// one deduplicated input-summary row: an input type (or the `(neutral)` /
/// `(unrecognized)` label) with its event count and decoded-value range.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectInputRow {
    pub index: i64,
    pub kind_name: &'static str,
    pub label: String,
    pub count: usize,
    pub value_range: Option<(i64, i64)>,
}

/// the observed raw range of each value register for one component, revealing
/// which bytes carry signal (including channels no input type decodes yet).
#[derive(Clone, Debug, PartialEq)]
pub struct InspectRegisterRow {
    pub index: i64,
    pub kind_name: &'static str,
    pub registers: [Option<(i64, i64)>; 8],
}

/// one host->device command summary row: a command signature (e.g.
/// `set_module i=0 field=2`) with its count and the range of its scalar value,
/// documenting the *output* protocol from a capture.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectOutputRow {
    pub signature: String,
    pub count: usize,
    pub value_range: Option<(i64, i64)>,
}

#[derive(Clone, Copy, Default)]
struct Stat {
    count: usize,
    range: Option<(i64, i64)>,
}

impl Stat {
    fn record(&mut self, value: Option<i64>) {
        self.count += 1;
        if let Some(v) = value {
            self.range = Some(extend(self.range, v));
        }
    }
}

struct ComponentInspect {
    kind_name: &'static str,
    inputs: BTreeMap<String, Stat>,
    registers: [Option<(i64, i64)>; 8],
}

impl ComponentInspect {
    fn new(kind_name: &'static str) -> ComponentInspect {
        ComponentInspect {
            kind_name,
            inputs: BTreeMap::new(),
            registers: [None; 8],
        }
    }
}

/// grow a running `(min, max)` range to include `value`.
fn extend(range: Option<(i64, i64)>, value: i64) -> (i64, i64) {
    match range {
        Some((lo, hi)) => (lo.min(value), hi.max(value)),
        None => (value, value),
    }
}

/// a running, deduplicated summary of inspected input events and (for capture
/// analysis) host->device output commands.
#[derive(Default)]
pub struct InspectAnalysis {
    components: BTreeMap<i64, ComponentInspect>,
    outputs: BTreeMap<String, Stat>,
}

impl InspectAnalysis {
    pub fn new() -> InspectAnalysis {
        InspectAnalysis::default()
    }

    /// fold one input event into the summary. `decoded` is the `(input type,
    /// value)` list the engine derived; when empty the event is recorded under
    /// its `(neutral)` / `(unrecognized)` label. every nonzero register widens
    /// that component's per-register range.
    pub fn observe_input(
        &mut self,
        index: i64,
        kind_name: &'static str,
        values: &[i64; 8],
        decoded: &[(&str, i64)],
    ) {
        let component = self
            .components
            .entry(index)
            .or_insert_with(|| ComponentInspect::new(kind_name));
        for (i, &value) in values.iter().enumerate() {
            if value != 0 {
                component.registers[i] = Some(extend(component.registers[i], value));
            }
        }
        if decoded.is_empty() {
            component
                .inputs
                .entry(undecoded_label(values).to_string())
                .or_default()
                .record(None);
        } else {
            for &(label, value) in decoded {
                component
                    .inputs
                    .entry(label.to_string())
                    .or_default()
                    .record(Some(value));
            }
        }
    }

    /// fold one host->device command into the output summary. a `set_module`
    /// led/config write is keyed by its `[index, field]` (so each addressed
    /// register is its own row, with the written value's range); any other
    /// command is keyed by its top-level key.
    pub fn observe_output(&mut self, message: &Value) {
        let Some(object) = message.as_object() else {
            return;
        };
        for (key, value) in object {
            let (signature, scalar) = match (key.as_str(), value) {
                ("set_module", Value::Array(a)) if a.len() == 3 => (
                    format!(
                        "set_module i={} field={}",
                        a[0].as_i64().unwrap_or(-1),
                        a[1].as_i64().unwrap_or(-1)
                    ),
                    a[2].as_i64(),
                ),
                _ => (key.clone(), value.as_i64()),
            };
            self.outputs.entry(signature).or_default().record(scalar);
        }
    }

    /// whether any input event has been observed.
    pub fn is_empty(&self) -> bool {
        self.components.is_empty()
    }

    /// the host->device command rows, ordered by signature.
    pub fn output_rows(&self) -> Vec<InspectOutputRow> {
        self.outputs
            .iter()
            .map(|(signature, stat)| InspectOutputRow {
                signature: signature.clone(),
                count: stat.count,
                value_range: stat.range,
            })
            .collect()
    }

    /// the deduplicated input rows, ordered by component then by label with the
    /// decoded types first and the `(...)` placeholders last.
    pub fn input_rows(&self) -> Vec<InspectInputRow> {
        let mut rows = Vec::new();
        for (&index, component) in &self.components {
            for (label, stat) in &component.inputs {
                rows.push(InspectInputRow {
                    index,
                    kind_name: component.kind_name,
                    label: label.clone(),
                    count: stat.count,
                    value_range: stat.range,
                });
            }
        }
        rows.sort_by(|a, b| {
            (a.index, a.label.starts_with('('), &a.label).cmp(&(
                b.index,
                b.label.starts_with('('),
                &b.label,
            ))
        });
        rows
    }

    /// the per-register raw activity rows, one per component, ordered by index.
    pub fn register_rows(&self) -> Vec<InspectRegisterRow> {
        self.components
            .iter()
            .map(|(&index, component)| InspectRegisterRow {
                index,
                kind_name: component.kind_name,
                registers: component.registers,
            })
            .collect()
    }
}

/// replay a decoded capture into an [`InspectAnalysis`]: device->host messages
/// rebuild the layout and drive the input summary (exactly as the live
/// controller decodes them), while host->device messages feed the output
/// summary. the two directions are decoded separately upstream; layout messages
/// precede inputs within the device->host stream, so order is preserved.
pub fn analyze_capture(device_to_host: &[Value], host_to_device: &[Value]) -> InspectAnalysis {
    let mut layout = ComponentLayout::new(0);
    let mut analysis = InspectAnalysis::new();
    for message in device_to_host {
        if let Some(tree) = message.get("l") {
            layout.update(tree);
        } else if let Some(inputs) = message.get("in").and_then(Value::as_array) {
            replay_input(&mut layout, &mut analysis, inputs);
        }
    }
    for message in host_to_device {
        analysis.observe_output(message);
    }
    analysis
}

/// decode one `{"in": [...]}` event against the layout and fold it into the
/// analysis, mirroring the live controller's `process_input` decode path.
fn replay_input(layout: &mut ComponentLayout, analysis: &mut InspectAnalysis, inputs: &[Value]) {
    for input in inputs {
        let (Some(index), Some(values)) = (
            input.get("i").and_then(Value::as_i64),
            input.get("v").and_then(Value::as_array),
        ) else {
            continue;
        };
        let mut v = [0i64; 8];
        for (slot, value) in v.iter_mut().zip(values) {
            *slot = value.as_i64().unwrap_or(0);
        }
        // a capture that starts mid-session carries no layout, so the component
        // kind is unknown; still record the raw register activity (the input is
        // left undecoded) so the per-register summary is useful regardless.
        let (kind_name, decoded) = match layout.find_component_mut(index) {
            Some(component) => {
                component.previous_values = component.values;
                component.values = v;
                let decoded = component
                    .determine_input_types()
                    .into_iter()
                    .map(|t| (t, component.dominant_value(t)))
                    .collect::<Vec<(&str, i64)>>();
                (component.kind_name(), decoded)
            }
            None => ("Unknown", Vec::new()),
        };
        analysis.observe_input(index, kind_name, &v, &decoded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn undecoded_label_separates_neutral_from_unknown() {
        assert_eq!("(neutral)", undecoded_label(&[0; 8]));
        assert_eq!("(unrecognized)", undecoded_label(&[8, 0, 0, 0, 0, 0, 0, 0]));
    }

    #[test]
    fn analysis_dedups_inputs_and_tracks_ranges_and_registers() {
        let mut a = InspectAnalysis::new();
        // two orbiter rotations of different magnitude, then a joystick push,
        // then an all-zero reset.
        a.observe_input(
            2,
            "Orbiter",
            &[1, 0, 0, 0, 0, 0, 0, 0],
            &[("rotation_right", 1)],
        );
        a.observe_input(
            2,
            "Orbiter",
            &[2, 0, 0, 0, 0, 0, 0, 0],
            &[("rotation_right", 2)],
        );
        a.observe_input(
            2,
            "Orbiter",
            &[0, 0, 91, 0, 253, 43, 119, 0],
            &[("joystick_x", 91), ("joystick_y", 253)],
        );
        a.observe_input(2, "Orbiter", &[0; 8], &[]);

        let inputs = a.input_rows();
        let rot = inputs.iter().find(|r| r.label == "rotation_right").unwrap();
        assert_eq!(2, rot.count);
        assert_eq!(Some((1, 2)), rot.value_range);
        let jx = inputs.iter().find(|r| r.label == "joystick_x").unwrap();
        assert_eq!(Some((91, 91)), jx.value_range);
        let neutral = inputs.iter().find(|r| r.label == "(neutral)").unwrap();
        assert_eq!(1, neutral.count);
        assert_eq!(None, neutral.value_range);

        // decoded types sort before the (neutral) placeholder.
        let labels: Vec<&str> = inputs.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(
            vec!["joystick_x", "joystick_y", "rotation_right", "(neutral)"],
            labels
        );

        // the third joystick channel (v5/v6) shows up as active registers even
        // though no input type decodes it.
        let regs = a.register_rows();
        assert_eq!(1, regs.len());
        let r = &regs[0].registers;
        assert_eq!(Some((1, 2)), r[0]); // rotation delta in v0
        assert_eq!(Some((91, 91)), r[2]); // joystick x low byte
        assert_eq!(Some((43, 43)), r[5]); // undecoded third channel
        assert_eq!(Some((119, 119)), r[6]);
        assert_eq!(None, r[7]);
    }

    #[test]
    fn observe_output_keys_set_module_by_index_and_field() {
        let mut a = InspectAnalysis::new();
        a.observe_output(&json!({"set_module": [0, 2, 16711680]}));
        a.observe_output(&json!({"set_module": [0, 2, 255]}));
        a.observe_output(&json!({"get_check": true}));
        a.observe_output(&json!({"clear_action": 2}));

        let rows = a.output_rows();
        let led = rows
            .iter()
            .find(|r| r.signature == "set_module i=0 field=2")
            .unwrap();
        assert_eq!(2, led.count);
        assert_eq!(Some((255, 16711680)), led.value_range);
        assert_eq!(
            1,
            rows.iter()
                .find(|r| r.signature == "get_check")
                .unwrap()
                .count
        );
    }

    #[test]
    fn analyze_capture_replays_layout_inputs_and_outputs() {
        // a core (kind 25) with a tri-dial child (kind 9, index 1), matching the
        // monogram layout shape.
        let layout = json!({"u":"core","i":0,"t":25,"c":[
            null, null, null,
            {"u":"tri","i":1,"t":9,"c":[null,null,null,null]},
            null
        ]});
        let device_to_host = vec![
            json!({"l": layout}),
            // tri-dial knob 2 turned right (v[2] = 5), then knob 1 pressed.
            json!({"in": [{"i": 1, "v": [0, 0, 5, 0, 0, 0, 0, 0]}, null, null, null]}),
            json!({"in": [{"i": 1, "v": [1, 0, 0, 0, 0, 0, 0, 0]}, null, null, null]}),
        ];
        let host_to_device = vec![json!({"set_module": [1, 5, 33488896]})];

        let a = analyze_capture(&device_to_host, &host_to_device);
        let inputs = a.input_rows();
        assert_eq!(
            Some((5, 5)),
            inputs
                .iter()
                .find(|r| r.label == "rotation_right_2")
                .unwrap()
                .value_range
        );
        assert_eq!(
            Some((1, 1)),
            inputs
                .iter()
                .find(|r| r.label == "pressed_1")
                .unwrap()
                .value_range
        );
        // the decode used the layout's kind, not the "Unknown" fallback.
        assert!(inputs.iter().all(|r| r.kind_name == "Tri-Dial"));

        let outputs = a.output_rows();
        assert_eq!("set_module i=1 field=5", outputs[0].signature);
        assert_eq!(Some((33488896, 33488896)), outputs[0].value_range);
    }
}
