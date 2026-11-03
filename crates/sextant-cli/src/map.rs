//! the `map` subcommand: program hid keyboard / midi note mappings onto a device.

use anyhow::Result;
use clap::Subcommand;
use std::time::Duration;

use crate::common::{run_command, Globals};

// settle time after programming a mapping before the process exits.
const MAP_SETTLE: Duration = Duration::from_secs(2);

#[derive(Subcommand)]
pub enum MapCommand {
    /// program hid keyboard mappings
    Hid,
    /// program midi note mappings
    Midi,
}

pub fn run(globals: &Globals, command: MapCommand) -> Result<()> {
    run_command(globals, |c| {
        c.wait_for_layout();
        match command {
            MapCommand::Hid => {
                c.configure_hidmap();
                c.check();
            }
            MapCommand::Midi => c.configure_midimap(),
        }
        std::thread::sleep(MAP_SETTLE);
        Ok(())
    })
}
