//! runtime configuration shared across the host crates.
//!
//! this mirrors the upstream global `FLAGS`. the cli builds it from clap; the
//! library treats it as plain data so it carries no argument-parser dependency.

/// the default device is monogram cc; palettegear is `16D0:09F8`.
pub const DEFAULT_DEVICE: &str = "0483:A2C5";

#[derive(Clone, Debug)]
pub struct Flags {
    pub device: String,
    pub device_index: usize,
    pub debug: bool,
    pub echo: bool,
    pub profiles: Vec<String>,
    pub map_midi: bool,
    pub config_paths: Vec<String>,
}

impl Default for Flags {
    fn default() -> Self {
        Flags {
            device: DEFAULT_DEVICE.to_string(),
            device_index: 0,
            debug: false,
            echo: false,
            profiles: Vec::new(),
            map_midi: true,
            config_paths: Vec::new(),
        }
    }
}
