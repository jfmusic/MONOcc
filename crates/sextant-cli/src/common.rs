//! shared cli plumbing: global flags, device + controller setup, and the
//! `ControllerOps` trait that lets a single command closure run against either
//! the fake device or the real backend.

use anyhow::{anyhow, Context, Result};
use clap::Args;
use std::io::BufRead;
use std::path::PathBuf;

use serde_json::Value;
use sextant_core::device::UsbError;
use sextant_core::{
    Configuration, Controller, InspectInputRow, InspectRegisterRow, Led, RenderComponent,
};
use sextant_fake::{layouts, FakeDevice};
use sextant_usb::UsbDevice;

/// the `--device` value that selects the in-memory fake device instead of real
/// hardware. the fake's behavior is tuned by the `simulator options` flags.
pub const SIMULATOR_DEVICE: &str = "simulator";

/// flags shared by every subcommand (declared `global` so they may appear before
/// or after the subcommand name).
#[derive(Args, Clone)]
pub struct Globals {
    /// target device: VENDOR:PRODUCT or `simulator` (auto-detected if unset)
    #[arg(long, global = true)]
    pub device: Option<String>,
    /// device index when several match
    #[arg(long, global = true, default_value_t = 0)]
    pub device_index: usize,
    /// named profile set to load (repeatable)
    #[arg(long = "profiles", global = true)]
    pub profiles: Vec<String>,
    /// profile file to load (repeatable)
    #[arg(long = "config", global = true)]
    pub config: Vec<PathBuf>,
    /// simulator layout: expertkit, prokit, base-only, monogram
    #[arg(
        long = "fake-layout",
        global = true,
        default_value = "prokit",
        help_heading = "simulator options"
    )]
    pub fake_layout: String,
    /// simulator firmware version
    #[arg(
        long = "fake-firmware",
        global = true,
        help_heading = "simulator options"
    )]
    pub fake_firmware: Option<String>,
    /// base-screen icon index override
    #[arg(long, global = true)]
    pub icon: Option<u8>,
    /// open the graphical display
    #[arg(long, global = true)]
    pub display: bool,
    /// echo input values to the base screen
    #[arg(long, global = true)]
    pub echo: bool,
    /// dump every device message
    #[arg(long, global = true)]
    pub debug: bool,
}

impl Globals {
    /// true when `--device simulator` selects the in-memory fake backend.
    pub fn is_simulator(&self) -> bool {
        self.device.as_deref() == Some(SIMULATOR_DEVICE)
    }
}

/// resolve a built-in layout name to its tree.
pub fn layout_by_name(name: &str) -> Result<Value> {
    Ok(match name {
        "expertkit" => layouts::expertkit(),
        "prokit" => layouts::prokit(),
        "base-only" | "base_only" => layouts::base_only(),
        "monogram" => layouts::monogram(),
        other => return Err(anyhow!("unknown layout {other:?}")),
    })
}

/// build the configuration from the named profile sets and explicit paths. the
/// device paths read profiles, screen images, logo, and font from the
/// configuration home, so this first requires that `sextant init` has run.
pub fn build_config(globals: &Globals) -> Result<Configuration> {
    if !sextant_core::resources::is_initialized() {
        return Err(anyhow!(
            "sextant is not initialized ({} missing); run `sextant init`",
            sextant_core::resources::config_home().display()
        ));
    }
    let mut config = if globals.profiles.is_empty() && globals.config.is_empty() {
        Configuration::default()
    } else {
        Configuration::from_sources(&globals.profiles, &globals.config)
            .context("loading profiles")?
    };
    if let Some(icon) = globals.icon {
        config.profile.icon = icon;
    }
    Ok(config)
}

/// a fake device preloaded with the requested layout and firmware version.
pub fn fake_device(globals: &Globals) -> Result<FakeDevice> {
    let mut device = FakeDevice::new("DEAD:BEEF");
    if let Some(version) = &globals.fake_firmware {
        device.set_version_core(version);
    }
    device.components_connected(layout_by_name(&globals.fake_layout)?);
    Ok(device)
}

pub fn usb_error(error: UsbError) -> anyhow::Error {
    anyhow!("{error}")
}

/// block until the user presses enter (or stdin closes), used to end a demo.
pub fn wait_for_enter() -> Result<()> {
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    Ok(())
}

/// the controller surface the device commands use, with the device type erased
/// so one closure serves both the fake and real backends.
pub trait ControllerOps {
    fn is_shutdown(&self) -> bool;
    fn rotate(&self, degrees: i32);
    fn layout_orientation(&self) -> i32;
    fn connected_count(&self) -> usize;
    fn connected_indices(&self) -> Vec<i64>;
    fn layout_extrema(&self) -> (i32, i32, i32, i32);
    fn render_components(&self) -> Vec<RenderComponent>;
    fn led(&self, leds: Vec<Led>);
    fn led_draw(&self, pixels: Vec<(u8, u8, u8)>, x: i32, y: i32, w: i32, h: i32);
    fn configure_leds(&self, red: Option<i64>, green: Option<i64>, blue: Option<i64>);
    fn screen_string(&self, text: &str);
    fn app_display_frame(&self, pixels: Vec<u8>) -> Result<(), String>;
    fn app_led_colors(&self, colors: Vec<(String, usize, (u8, u8, u8))>);
    fn configure_images(&self);
    fn upload_images(&self, images: &[(i64, Vec<u8>)]);
    fn configure_hidmap(&self);
    fn configure_midimap(&self);
    fn boot(&self);
    fn check(&self);
    fn wait(&self);
    fn wait_for_layout(&self);
    fn print_stats(&self);
    fn inspect_summary(&self) -> (Vec<InspectInputRow>, Vec<InspectRegisterRow>);
}

impl<D: sextant_core::Device> ControllerOps for Controller<D> {
    fn is_shutdown(&self) -> bool { Controller::<D>::is_shutdown(self) }
    fn rotate(&self, degrees: i32) { Controller::<D>::rotate(self, degrees) }
    fn layout_orientation(&self) -> i32 { Controller::<D>::layout_orientation(self) }
    fn connected_count(&self) -> usize {
        Controller::<D>::connected_count(self)
    }
    fn connected_indices(&self) -> Vec<i64> {
        Controller::<D>::connected_indices(self)
    }
    fn layout_extrema(&self) -> (i32, i32, i32, i32) {
        Controller::<D>::layout_extrema(self)
    }
    fn render_components(&self) -> Vec<RenderComponent> {
        Controller::<D>::render_components(self)
    }
    fn led(&self, leds: Vec<Led>) {
        Controller::<D>::led(self, leds)
    }
    fn led_draw(&self, pixels: Vec<(u8, u8, u8)>, x: i32, y: i32, w: i32, h: i32) {
        Controller::<D>::led_draw(self, pixels, x, y, w, h)
    }
    fn configure_leds(&self, red: Option<i64>, green: Option<i64>, blue: Option<i64>) {
        Controller::<D>::configure_leds(self, red, green, blue)
    }
    fn screen_string(&self, text: &str) {
        Controller::<D>::screen_string(self, text)
    }
    fn app_display_frame(&self, pixels: Vec<u8>) -> Result<(), String> {
        Controller::<D>::app_display_frame(self, pixels)
    }
    fn app_led_colors(&self, colors: Vec<(String, usize, (u8, u8, u8))>) {
        Controller::<D>::app_led_colors(self, colors)
    }
    fn configure_images(&self) {
        Controller::<D>::configure_images(self)
    }
    fn upload_images(&self, images: &[(i64, Vec<u8>)]) {
        Controller::<D>::upload_images(self, images)
    }
    fn configure_hidmap(&self) {
        Controller::<D>::configure_hidmap(self)
    }
    fn configure_midimap(&self) {
        Controller::<D>::configure_midimap(self)
    }
    fn boot(&self) {
        Controller::<D>::boot(self)
    }
    fn check(&self) {
        Controller::<D>::check(self)
    }
    fn wait(&self) {
        Controller::<D>::wait(self)
    }
    fn wait_for_layout(&self) {
        Controller::<D>::wait_for_layout(self)
    }
    fn print_stats(&self) {
        Controller::<D>::print_stats(self)
    }
    fn inspect_summary(&self) -> (Vec<InspectInputRow>, Vec<InspectRegisterRow>) {
        Controller::<D>::inspect_summary(self)
    }
}

/// resolve the real device from the recognized listing: an explicit `--device`
/// narrows to that id, and `--device-index` selects the nth match in listing
/// order (the order `sextant devices` prints), so the right wire format is used.
pub(crate) fn open_usb_device(globals: &Globals) -> Result<UsbDevice> {
    let devices = sextant_usb::list_devices().map_err(usb_error)?;
    let (id, offset) =
        sextant_usb::select_device(&devices, globals.device.as_deref(), globals.device_index)
            .ok_or_else(|| {
                anyhow!(
                    "no matching palette / monogram device at index {}; run `sextant devices`",
                    globals.device_index
                )
            })?;
    UsbDevice::new(&id, offset).map_err(usb_error)
}

/// build, open, and connect a controller over the real device, waiting for its
/// layout, so the gui can render the live hardware.
pub fn open_device_controller(globals: &Globals) -> Result<Controller<UsbDevice>> {
    let config = build_config(globals)?;
    let device = open_usb_device(globals)?;
    let mut controller = Controller::new(config, device);
    controller.set_echo(globals.echo);
    controller.set_debug(globals.debug);
    controller.open().map_err(usb_error)?;
    controller.connect().map_err(usb_error)?;
    controller.wait_for_layout();
    Ok(controller)
}

/// build and connect a controller (fake or real per `--device simulator`), run
/// the command against it, then close it.
pub fn run_command(
    globals: &Globals,
    command: impl FnOnce(&dyn ControllerOps) -> Result<()>,
) -> Result<()> {
    run_command_inner(globals, false, command)
}

/// like [`run_command`], but with the controller in inspect mode so every input
/// event prints its decoded one-line summary (used by the `inspect` command).
pub fn run_inspect(
    globals: &Globals,
    command: impl FnOnce(&dyn ControllerOps) -> Result<()>,
) -> Result<()> {
    run_command_inner(globals, true, command)
}

fn run_command_inner(
    globals: &Globals,
    inspect: bool,
    command: impl FnOnce(&dyn ControllerOps) -> Result<()>,
) -> Result<()> {
    let config = build_config(globals)?;
    if globals.is_simulator() {
        drive(
            Controller::new(config, fake_device(globals)?),
            globals,
            inspect,
            command,
        )
    } else {
        drive(
            Controller::new(config, open_usb_device(globals)?),
            globals,
            inspect,
            command,
        )
    }
}

/// configure, open, run against, and close a controller of either backend.
fn drive<D: sextant_core::Device>(
    mut controller: Controller<D>,
    globals: &Globals,
    inspect: bool,
    command: impl FnOnce(&dyn ControllerOps) -> Result<()>,
) -> Result<()> {
    controller.set_echo(globals.echo);
    controller.set_debug(globals.debug);
    controller.set_inspect(inspect);
    controller.open().map_err(usb_error)?;
    controller.connect().map_err(usb_error)?;
    let result = command(&controller);
    controller.close();
    result
}
