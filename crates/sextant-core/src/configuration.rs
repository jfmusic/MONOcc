//! profile configuration parsed from paletteapp `.plp` json.
//!
//! a profile binds component inputs to actions (the `actionMap`), media keys
//! (the `mediaMap`), and raw hid keys (the `keyMap`), and carries a layout and
//! base orientation. a [`Configuration`] holds an ordered set of profiles and
//! the index of the active one; profile switching wraps around.

use serde_json::{Map, Value};
use std::path::Path;

/// action identifiers used as `"action"` values in an `actionMap` binding.
pub mod action {
    pub const PROFILE_PREV: &str = "previousFunction";
    pub const PROFILE_NEXT: &str = "nextFunction";
    pub const PROFILE_SWITCH: &str = "switchFunctions";
    pub const TOGGLE_PREVIEW: &str = "togglePreview";
    pub const SET_COLOR_RED: &str = "setColorRed";
    pub const SET_COLOR_GREEN: &str = "setColorGreen";
    pub const SET_COLOR_BLUE: &str = "setColorBlue";
    pub const EXECUTE_COMMAND: &str = "executeCommand";
    pub const CYCLE_MAPPING: &str = "cycleMapping";
    pub const TEST_CYCLE: &str = "testCycle";
    pub const TEST_ACCUMULATE: &str = "testAccumulate";
    pub const TEST_DELTA: &str = "testDelta";
    pub const TEST_TRIGGER: &str = "testTrigger";
}

/// input-event identifiers, used as keys within a component's binding map.
pub mod input {
    pub const PRESSED: &str = "pressed";
    pub const RELEASED: &str = "released";
    pub const LONG_PRESSED: &str = "long_pressed";
    pub const DOUBLE_PRESSED: &str = "double_pressed";
    pub const TRIPLE_PRESSED: &str = "triple_pressed";
    pub const ROTATION_RIGHT: &str = "rotation_right";
    pub const ROTATION_LEFT: &str = "rotation_left";
    pub const ROTATION: &str = "rotation";
    pub const PRESSED_ROTATION_RIGHT: &str = "pressed_rotation_right";
    pub const PRESSED_ROTATION_LEFT: &str = "pressed_rotation_left";
    pub const PRESSED_ROTATION: &str = "pressed_rotation";
    pub const SET: &str = "set";

    // indexed variants for multi-control monogram modules: the tri-dial's three
    // knobs and the core's / tri-dial's individually-addressable buttons. a
    // trailing `_<n>` selects the sub-control; the engine reads the base type
    // (see `split_index` in component.rs), so behavior is identical to the
    // unindexed forms. the orbiter's joystick axes are exposed as set inputs.
    pub const PRESSED_1: &str = "pressed_1";
    pub const PRESSED_2: &str = "pressed_2";
    pub const PRESSED_3: &str = "pressed_3";
    pub const RELEASED_1: &str = "released_1";
    pub const RELEASED_2: &str = "released_2";
    pub const RELEASED_3: &str = "released_3";
    pub const ROTATION_LEFT_1: &str = "rotation_left_1";
    pub const ROTATION_LEFT_2: &str = "rotation_left_2";
    pub const ROTATION_LEFT_3: &str = "rotation_left_3";
    pub const ROTATION_RIGHT_1: &str = "rotation_right_1";
    pub const ROTATION_RIGHT_2: &str = "rotation_right_2";
    pub const ROTATION_RIGHT_3: &str = "rotation_right_3";
    // each tri-dial knob also exposes an absolute `set` position, integrated
    // host-side from its rotation deltas (the device reports deltas only), so it
    // can drive absolute bindings like a palettegear dial's `set`.
    pub const SET_1: &str = "set_1";
    pub const SET_2: &str = "set_2";
    pub const SET_3: &str = "set_3";
    pub const JOYSTICK_X: &str = "joystick_x";
    pub const JOYSTICK_Y: &str = "joystick_y";
}

/// base-screen icon indices; these map onto embedded `.raw` assets by number.
pub mod icon {
    pub const PALETTE: u8 = 0;
    pub const PHOTOSHOP: u8 = 1;
    pub const LIGHTROOM: u8 = 2;
    pub const AE: u8 = 3;
    pub const MIDI: u8 = 4;
    pub const KEYS: u8 = 5;
    pub const AI: u8 = 6;
    pub const JOYSTICK: u8 = 7;
    pub const COPYRIGHT: u8 = 8;
    pub const ID: u8 = 9;
    pub const PR: u8 = 10;
    pub const VIDEO: u8 = 11;
    pub const AU: u8 = 12;
    pub const CAPITAL_ONE: u8 = 13;
    pub const CHROME: u8 = 14;
    pub const SPOTIFY: u8 = 15;
    pub const MUSIC: u8 = 16;
    pub const VLC: u8 = 17;
    pub const CH: u8 = 18;
    pub const MEDIA: u8 = 19;
    pub const FLT: u8 = 20;
    pub const GEAR: u8 = 21;
    pub const BLANK: u8 = 23;
    pub const HOME: u8 = 24;
}

/// map a paletteapp `tabType` to its base-screen icon.
pub fn icon_for_tab(tab_type: &str) -> Option<u8> {
    Some(match tab_type {
        "Keyboard" => icon::KEYS,
        "Joystick" => icon::JOYSTICK,
        "MIDI" => icon::MIDI,
        "Palette" => icon::PALETTE,
        "Video" => icon::VIDEO,
        "Settings" => icon::GEAR,
        "Media" => icon::MEDIA,
        "Music" => icon::MUSIC,
        "Home" => icon::HOME,
        _ => return None,
    })
}

/// translate a qt modifier keycode into its `PK_MOD_*` name.
pub fn qt_modifier(code: u64) -> Option<&'static str> {
    Some(match code {
        0x01000020 => "PK_MOD_LSHIFT",
        0x01000021 => "PK_MOD_LCTRL",
        0x01000022 => "PK_MOD_LMETA",
        0x01000023 => "PK_MOD_LALT",
        _ => return None,
    })
}

/// translate a `PK_*` usb hid key name into its usage code.
pub fn hid_key(name: &str) -> Option<u8> {
    Some(match name {
        "PK_MOD_LCTRL" => 0x01,
        "PK_MOD_LSHIFT" => 0x02,
        "PK_MOD_LALT" => 0x04,
        "PK_MOD_LMETA" => 0x08,
        "PK_MOD_RCTRL" => 0x10,
        "PK_MOD_RSHIFT" => 0x20,
        "PK_MOD_RALT" => 0x40,
        "PK_MOD_RMETA" => 0x80,
        "PK_NONE" => 0x00,
        "PK_ERR_OVF" => 0x01,
        "PK_A" => 0x04,
        "PK_B" => 0x05,
        "PK_C" => 0x06,
        "PK_D" => 0x07,
        "PK_E" => 0x08,
        "PK_F" => 0x09,
        "PK_G" => 0x0a,
        "PK_H" => 0x0b,
        "PK_I" => 0x0c,
        "PK_J" => 0x0d,
        "PK_K" => 0x0e,
        "PK_L" => 0x0f,
        "PK_M" => 0x10,
        "PK_N" => 0x11,
        "PK_O" => 0x12,
        "PK_P" => 0x13,
        "PK_Q" => 0x14,
        "PK_R" => 0x15,
        "PK_S" => 0x16,
        "PK_T" => 0x17,
        "PK_U" => 0x18,
        "PK_V" => 0x19,
        "PK_W" => 0x1a,
        "PK_X" => 0x1b,
        "PK_Y" => 0x1c,
        "PK_Z" => 0x1d,
        "PK_1" => 0x1e,
        "PK_2" => 0x1f,
        "PK_3" => 0x20,
        "PK_4" => 0x21,
        "PK_5" => 0x22,
        "PK_6" => 0x23,
        "PK_7" => 0x24,
        "PK_8" => 0x25,
        "PK_9" => 0x26,
        "PK_0" => 0x27,
        "PK_RETURN" => 0x28,
        "PK_ESCAPE" => 0x29,
        "PK_BACKSPACE" => 0x2a,
        "PK_TAB" => 0x2b,
        "PK_SPACE" => 0x2c,
        "PK_MINUS" => 0x2d,
        "PK_EQUAL" => 0x2e,
        "PK_LEFTBRACE" => 0x2f,
        "PK_RIGHTBRACE" => 0x30,
        "PK_BACKSLASH" => 0x31,
        "PK_HASHTILDE" => 0x32,
        "PK_SEMICOLON" => 0x33,
        "PK_APOSTROPHE" => 0x34,
        "PK_GRAVE" => 0x35,
        "PK_COMMA" => 0x36,
        "PK_DOT" => 0x37,
        "PK_SLASH" => 0x38,
        "PK_CAPSLOCK" => 0x39,
        "PK_F1" => 0x3a,
        "PK_F2" => 0x3b,
        "PK_F3" => 0x3c,
        "PK_F4" => 0x3d,
        "PK_F5" => 0x3e,
        "PK_F6" => 0x3f,
        "PK_F7" => 0x40,
        "PK_F8" => 0x41,
        "PK_F9" => 0x42,
        "PK_F10" => 0x43,
        "PK_F11" => 0x44,
        "PK_F12" => 0x45,
        "PK_SYSRQ" => 0x46,
        "PK_SCROLLLOCK" => 0x47,
        "PK_PAUSE" => 0x48,
        "PK_INSERT" => 0x49,
        "PK_HOME" => 0x4a,
        "PK_PAGEUP" => 0x4b,
        "PK_DELETE" => 0x4c,
        "PK_END" => 0x4d,
        "PK_PAGEDOWN" => 0x4e,
        "PK_RIGHT" => 0x4f,
        "PK_LEFT" => 0x50,
        "PK_DOWN" => 0x51,
        "PK_UP" => 0x52,
        "PK_NUMLOCK" => 0x53,
        "PK_KPSLASH" => 0x54,
        "PK_KPASTERISK" => 0x55,
        "PK_KPMINUS" => 0x56,
        "PK_KPPLUS" => 0x57,
        "PK_KPENTER" => 0x58,
        "PK_KP1" => 0x59,
        "PK_KP2" => 0x5a,
        "PK_KP3" => 0x5b,
        "PK_KP4" => 0x5c,
        "PK_KP5" => 0x5d,
        "PK_KP6" => 0x5e,
        "PK_KP7" => 0x5f,
        "PK_KP8" => 0x60,
        "PK_KP9" => 0x61,
        "PK_KP0" => 0x62,
        "PK_KPDOT" => 0x63,
        "PK_102ND" => 0x64,
        "PK_COMPOSE" => 0x65,
        "PK_POWER" => 0x66,
        "PK_KPEQUAL" => 0x67,
        "PK_F13" => 0x68,
        "PK_F14" => 0x69,
        "PK_F15" => 0x6a,
        "PK_F16" => 0x6b,
        "PK_F17" => 0x6c,
        "PK_F18" => 0x6d,
        "PK_F19" => 0x6e,
        "PK_F20" => 0x6f,
        "PK_F21" => 0x70,
        "PK_F22" => 0x71,
        "PK_F23" => 0x72,
        "PK_F24" => 0x73,
        "PK_OPEN" => 0x74,
        "PK_HELP" => 0x75,
        "PK_PROPS" => 0x76,
        "PK_FRONT" => 0x77,
        "PK_STOP" => 0x78,
        "PK_AGAIN" => 0x79,
        "PK_UNDO" => 0x7a,
        "PK_CUT" => 0x7b,
        "PK_COPY" => 0x7c,
        "PK_PASTE" => 0x7d,
        "PK_FIND" => 0x7e,
        "PK_MUTE" => 0x7f,
        "PK_VOLUMEUP" => 0x80,
        "PK_VOLUMEDOWN" => 0x81,
        "PK_KPCOMMA" => 0x85,
        "PK_RO" => 0x87,
        "PK_KATAKANAHIRAGANA" => 0x88,
        "PK_YEN" => 0x89,
        "PK_HENKAN" => 0x8a,
        "PK_MUHENKAN" => 0x8b,
        "PK_KPJPCOMMA" => 0x8c,
        "PK_HANGEUL" => 0x90,
        "PK_HANJA" => 0x91,
        "PK_KATAKANA" => 0x92,
        "PK_HIRAGANA" => 0x93,
        "PK_ZENKAKUHANKAKU" => 0x94,
        "PK_KPLEFTPAREN" => 0xb6,
        "PK_KPRIGHTPAREN" => 0xb7,
        "PK_LEFTCTRL" => 0xe0,
        "PK_LEFTSHIFT" => 0xe1,
        "PK_LEFTALT" => 0xe2,
        "PK_LEFTWINDOWS" => 0xe3,
        "PK_NOT_A_KEY" => 0xe3,
        "PK_RIGHTCTRL" => 0xe4,
        "PK_RIGHTSHIFT" => 0xe5,
        "PK_RIGHTALT" => 0xe6,
        "PK_RIGHTMETA" => 0xe7,
        "PK_MEDIA_PLAYPAUSE" => 0xe8,
        "PK_MEDIA_STOPCD" => 0xe9,
        "PK_MEDIA_PREVIOUSSONG" => 0xea,
        "PK_MEDIA_NEXTSONG" => 0xeb,
        "PK_MEDIA_EJECTCD" => 0xec,
        "PK_MEDIA_VOLUMEUP" => 0xed,
        "PK_MEDIA_VOLUMEDOWN" => 0xee,
        "PK_MEDIA_MUTE" => 0xef,
        "PK_MEDIA_WWW" => 0xf0,
        "PK_MEDIA_BACK" => 0xf1,
        "PK_MEDIA_FORWARD" => 0xf2,
        "PK_MEDIA_STOP" => 0xf3,
        "PK_MEDIA_FIND" => 0xf4,
        "PK_MEDIA_SCROLLUP" => 0xf5,
        "PK_MEDIA_SCROLLDOWN" => 0xf6,
        "PK_MEDIA_EDIT" => 0xf7,
        "PK_MEDIA_SLEEP" => 0xf8,
        "PK_MEDIA_COFFEE" => 0xf9,
        "PK_MEDIA_REFRESH" => 0xfa,
        "PK_MEDIA_CALC" => 0xfb,
        _ => return None,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read {0}: {1}")]
    Read(String, std::io::Error),
    #[error("invalid json in {0}: {1}")]
    Json(String, serde_json::Error),
    #[error("unknown tabType {0:?} in {1}")]
    UnknownTab(String, String),
    #[error("profile set {0:?} not found in the configuration home; run `sextant init`")]
    MissingProfileSet(String),
}

/// a single hid binding: the control name and the (key, modifier, repeat) it
/// programs onto the device.
pub type HidKey = (String, u8, u8, u8);

/// a single configured action: input event, action name, and its raw config.
pub type ComponentAction = (String, String, Value);

#[derive(Clone, Debug)]
pub struct Profile {
    pub title: String,
    pub icon: u8,
    pub action_map: Map<String, Value>,
    pub media_map: Map<String, Value>,
    pub key_map: Map<String, Value>,
    pub layout: Value,
    pub orientation: i32,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            title: "SEXTANT".to_string(),
            // a clean black base screen by default (slot 23); no brand icon is
            // bundled.
            icon: icon::BLANK,
            action_map: Map::new(),
            media_map: Map::new(),
            key_map: Map::new(),
            layout: Value::Null,
            orientation: 0,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Configuration {
    pub profile_index: usize,
    pub profile: Profile,
    pub profiles: Vec<Profile>,
}

fn object_field(map: &Map<String, Value>, key: &str) -> Map<String, Value> {
    map.get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

impl Configuration {
    /// load and parse a set of `.plp` files into ordered profiles, activating
    /// the first. an empty path set yields a single default profile.
    pub fn load(paths: &[impl AsRef<Path>]) -> Result<Configuration, ConfigError> {
        let mut config = Configuration::default();
        if paths.is_empty() {
            return Ok(config);
        }
        for path in paths {
            let name = path.as_ref().display().to_string();
            let bytes = std::fs::read_to_string(path.as_ref())
                .map_err(|e| ConfigError::Read(name.clone(), e))?;
            config.add_profile(&bytes, &name)?;
        }
        config.switch(0);
        Ok(config)
    }

    /// build a configuration from named profile sets plus explicit `.plp` file
    /// paths, activating the first profile. mirrors the upstream
    /// `StandardConfiguration` (the `--profiles` / config-path surface). a named
    /// set is read from the configuration home (`<home>/profiles/NAME/`); an
    /// absent set is an error, since the home must be seeded with `sextant init`.
    pub fn from_sources(
        profiles: &[String],
        paths: &[impl AsRef<Path>],
    ) -> Result<Configuration, ConfigError> {
        let mut config = Configuration::default();
        let mut count = 0;
        for name in profiles {
            let user_paths = crate::resources::user_profile_paths(name);
            if user_paths.is_empty() {
                return Err(ConfigError::MissingProfileSet(name.clone()));
            }
            for path in user_paths {
                let display = path.display().to_string();
                let bytes = std::fs::read_to_string(&path)
                    .map_err(|e| ConfigError::Read(display.clone(), e))?;
                config.add_profile(&bytes, &display)?;
                count += 1;
            }
        }
        for path in paths {
            let name = path.as_ref().display().to_string();
            let bytes = std::fs::read_to_string(path.as_ref())
                .map_err(|e| ConfigError::Read(name.clone(), e))?;
            config.add_profile(&bytes, &name)?;
            count += 1;
        }
        if count > 0 {
            config.switch(0);
        }
        Ok(config)
    }

    /// parse one `.plp` document and append it as a profile.
    pub fn add_profile(&mut self, bytes: &str, name: &str) -> Result<(), ConfigError> {
        let dict: Value =
            serde_json::from_str(bytes).map_err(|e| ConfigError::Json(name.to_string(), e))?;
        let tab = dict.get("tabType").and_then(Value::as_str).unwrap_or("");
        let icon =
            icon_for_tab(tab).ok_or_else(|| ConfigError::UnknownTab(tab.into(), name.into()))?;
        let mappings = dict
            .get("module_mappings")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        self.profiles.push(Profile {
            title: dict
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            icon,
            action_map: object_field(&mappings, "actionMap"),
            media_map: object_field(&mappings, "mediaMap"),
            key_map: object_field(&mappings, "keyMap"),
            layout: dict.get("layout").cloned().unwrap_or(Value::Null),
            orientation: dict.get("orientation").and_then(Value::as_i64).unwrap_or(0) as i32,
        });
        Ok(())
    }

    pub fn switch(&mut self, index: usize) {
        self.profile_index = index;
        self.profile = self.profiles[index].clone();
    }

    pub fn prev(&mut self) {
        let index = if self.profile_index == 0 {
            self.profiles.len() - 1
        } else {
            self.profile_index - 1
        };
        self.switch(index);
    }

    pub fn next(&mut self) {
        let index = if self.profile_index + 1 >= self.profiles.len() {
            0
        } else {
            self.profile_index + 1
        };
        self.switch(index);
    }

    /// the hid bindings configured for a component uid: keyboard usages from the
    /// `keyMap` (`virtual_code`) plus consumer / media usages from the `mediaMap`
    /// (`key`, a `PK_MEDIA_*` name). both are programmed onto the device hidmap so
    /// the device emits the usage; the host cannot send hid itself.
    pub fn component_hidkeys(&self, uid: &str) -> Vec<HidKey> {
        let mut out = Vec::new();
        let mut collect = |map: &Map<String, Value>, key_field: &str| {
            let Some(controls) = map.get(uid).and_then(Value::as_object) else {
                return;
            };
            for (control, value) in controls {
                let Some(bind) = value.as_object() else {
                    continue;
                };
                let Some(key) = bind
                    .get(key_field)
                    .and_then(Value::as_str)
                    .and_then(hid_key)
                else {
                    continue;
                };
                let mut modifier: u8 = 0;
                if let Some(codes) = bind.get("modifier_codes").and_then(Value::as_array) {
                    for code in codes {
                        let name = match code {
                            Value::Number(n) => n.as_u64().and_then(qt_modifier),
                            Value::String(s) => Some(s.as_str()),
                            _ => None,
                        };
                        if let Some(bits) = name.and_then(hid_key) {
                            modifier = modifier.wrapping_add(bits);
                        }
                    }
                }
                let repeat = match bind.get("dial_sensitivity").and_then(Value::as_i64) {
                    Some(1) | None => 1,
                    Some(_) => 8,
                };
                out.push((control.clone(), key, modifier, repeat));
            }
        };
        collect(&self.profile.key_map, "virtual_code");
        collect(&self.profile.media_map, "key");
        out
    }

    /// sorted set of component uid-queries that have any action binding.
    pub fn components(&self) -> Vec<String> {
        let mut matchers: Vec<String> = self.profile.action_map.keys().cloned().collect();
        matchers.sort();
        matchers.dedup();
        matchers
    }

    /// the configured actions for a component uid, from the `actionMap`. (media
    /// bindings are hid output, programmed via [`Self::component_hidkeys`].)
    pub fn component_actions(&self, uid: &str) -> Vec<ComponentAction> {
        let mut out = Vec::new();
        if let Some(bindings) = self.profile.action_map.get(uid).and_then(Value::as_object) {
            for (input_type, config) in bindings {
                let Some(action_name) = config.get("action").and_then(Value::as_str) else {
                    continue;
                };
                out.push((input_type.clone(), action_name.to_string(), config.clone()));
            }
        }
        out
    }

    pub fn set_component_actions(&mut self, uid: &str, mapping: Value) {
        self.profile.action_map.insert(uid.to_string(), mapping);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_and_key_lookups() {
        assert_eq!(Some(icon::PALETTE), icon_for_tab("Palette"));
        assert_eq!(Some(icon::JOYSTICK), icon_for_tab("Joystick"));
        assert_eq!(None, icon_for_tab("Nope"));
        assert_eq!(Some(0x04), hid_key("PK_A"));
        assert_eq!(Some("PK_MOD_LCTRL"), qt_modifier(0x01000021));
    }

    #[test]
    fn parses_meta_profile() {
        let bytes = include_str!("../resources/profiles/multifunction-buttons/00-meta.plp");
        let mut config = Configuration::default();
        config.add_profile(bytes, "00-meta.plp").unwrap();
        config.switch(0);
        assert_eq!("META", config.profile.title);
        assert_eq!(icon::PALETTE, config.profile.icon);

        // bindings without an "action" are filtered out.
        let prev = config.component_actions("[slot=(-1,0)]");
        assert_eq!(1, prev.len());
        assert_eq!(input::RELEASED, prev[0].0);
        assert_eq!(action::PROFILE_PREV, prev[0].1);
    }

    #[test]
    fn mediamap_programs_consumer_hid_keys() {
        // a mediaMap binding is a consumer/media usage the device emits, mirroring
        // the keyMap; it is folded into the component's hid bindings.
        let media_map: Map<String, Value> =
            serde_json::from_str(r#"{"4FH": {"pressed": {"key": "PK_MEDIA_PLAYPAUSE"}}}"#).unwrap();
        let mut config = Configuration {
            profiles: vec![Profile {
                media_map,
                ..Default::default()
            }],
            ..Default::default()
        };
        config.switch(0);
        let keys = config.component_hidkeys("4FH");
        assert_eq!(1, keys.len());
        assert_eq!("pressed", keys[0].0.as_str());
        assert_eq!(0xe8, keys[0].1); // PK_MEDIA_PLAYPAUSE consumer usage
        assert_eq!(0, keys[0].2); // no modifier
    }

    #[test]
    fn profile_switching_wraps() {
        let mut config = Configuration {
            profiles: vec![
                Profile {
                    title: "A".into(),
                    ..Default::default()
                },
                Profile {
                    title: "B".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        config.switch(0);
        config.prev();
        assert_eq!("B", config.profile.title);
        config.next();
        assert_eq!("A", config.profile.title);
    }
}
