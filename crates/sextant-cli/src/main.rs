//! sextant: the command-line entry point.
//!
//! one binary with subcommands replaces the upstream's many `bin/ambit_*`
//! scripts; selecting `--device simulator` runs against the in-memory fake
//! device, replacing the duplicated `_simulator` variants.

mod common;
mod demos;
mod headless;
mod map;
mod pcap;
mod probe;
mod profile_bridge;
mod resource;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use common::{
    build_config, layout_by_name, run_command, run_inspect, wait_for_enter, ControllerOps, Globals,
};
use sextant_core::message::decode_msgpack_lenient;
use sextant_core::{
    inspect, Configuration, InspectInputRow, InspectOutputRow, InspectRegisterRow, Led,
};

#[derive(Parser)]
#[command(
    name = "sextant",
    about = "take control of your palette / monogram device"
)]
struct Cli {
    #[command(flatten)]
    globals: Globals,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// list connected devices
    Devices,
    /// connect and run the input loop
    ///
    /// add `--display` for the graphical interface.
    Connect,
    /// list available profile sets
    Profiles,
    /// extract the bundled resources to ~/.config/sextant
    ///
    /// device commands read profiles and screen assets from there, so run this
    /// once first. existing files are kept unless `--force`.
    Init {
        /// overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// run an led demo
    #[command(subcommand)]
    Demo(DemoCommand),
    /// program hid / midi mappings
    #[command(subcommand)]
    Map(map::MapCommand),
    /// convert, display, or upload screen images
    #[command(subcommand)]
    Resource(resource::ResourceCommand),
    /// developer and debugging tools
    #[command(subcommand)]
    Tools(ToolsCommand),
}

#[derive(Subcommand)]
enum DemoCommand {
    /// scrolling sinebow with dial / button control
    Sinebow,
    /// drifting pastel per-component glow
    Lavalamp,
    /// random single-component flashes
    Lightshow,
}

#[derive(Subcommand)]
enum ToolsCommand {
    /// native macOS profile editor connection
    ProfileBridge,
    /// interactive session against a device
    Console,
    /// decode inputs live, or analyze a `.pcapng` offline
    Inspect {
        /// a usbmon `.pcapng` capture to analyze offline (omit for live capture)
        capture: Option<PathBuf>,
    },
    /// measure led / screen throughput
    Benchmark,
    /// find the safe bulk-write rate
    ///
    /// ratchets the rate up to where the device core resets.
    Probe,
    /// scriptable device-free mode (used by the e2e tests)
    Headless,
    /// reboot into the dfu bootloader
    RebootBootloader,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let globals = &cli.globals;
    match cli.command {
        Command::Devices => list_devices(),
        // `--display` opens the gui; otherwise run the headless input/keepalive loop.
        Command::Connect if globals.display => open_display(globals),
        Command::Connect => {
            // the default connect-and-run mode; pass --echo to mirror the adjusted
            // input's value onto the base screen.
            run_command(globals, |c| {
                c.wait_for_layout();
                eprintln!("connected. {} component(s):", c.connected_count());
                for rc in c.render_components() {
                    eprintln!(
                        "  index={} uid={:?} kind={} ({}) slot={:?} orientation={} values={:?}",
                        rc.index, rc.uid, rc.kind, rc.kind_name, rc.slot, rc.orientation, rc.values
                    );
                }
                eprintln!("press enter to stop.");
                wait_for_enter()?;
                c.print_stats();
                Ok(())
            })
        }
        Command::Profiles => list_profiles(),
        Command::Init { force } => run_init(force),
        Command::Demo(command) => run_demo(globals, command),
        Command::Map(command) => map::run(globals, command),
        Command::Resource(command) => resource::run(globals, command),
        Command::Tools(command) => run_tools(globals, command),
    }
}

fn run_demo(globals: &Globals, command: DemoCommand) -> Result<()> {
    // `--display` watches the demo in the gui, driving the connected device each
    // frame -- the live device by default, or the fake with `--device simulator`.
    // without it, the demo drives the device's leds (fake or real) headless.
    if globals.display {
        return match command {
            DemoCommand::Sinebow => {
                let mut demo = demos::Demoscene::new();
                display_demo(globals, move |c| demo.step(c))
            }
            DemoCommand::Lavalamp => {
                let mut demo = demos::Lavalamp::new();
                display_demo(globals, move |c| demo.step(c))
            }
            DemoCommand::Lightshow => {
                let mut demo = demos::Lightshow::new();
                display_demo(globals, move |c| demo.step(c))
            }
        };
    }
    match command {
        DemoCommand::Sinebow => run_command(globals, |c| {
            c.wait_for_layout();
            demos::demoscene(c);
            Ok(())
        }),
        DemoCommand::Lavalamp => run_command(globals, |c| {
            // lavalamp snapshots the connected components once, so the layout
            // must be in before it starts or it animates nothing.
            c.wait_for_layout();
            demos::lavalamp(c);
            Ok(())
        }),
        DemoCommand::Lightshow => run_command(globals, |c| {
            c.wait_for_layout();
            demos::lightshow(c);
            Ok(())
        }),
    }
}

fn run_tools(globals: &Globals, command: ToolsCommand) -> Result<()> {
    match command {
        ToolsCommand::Console => run_command(globals, |c| {
            c.wait_for_layout();
            eprintln!("console: scripted control is available via `sextant tools headless`.");
            eprintln!("press enter to stop.");
            wait_for_enter()?;
            c.print_stats();
            Ok(())
        }),
        ToolsCommand::Inspect {
            capture: Some(path),
        } => inspect_pcap(&path),
        ToolsCommand::Inspect { capture: None } => run_inspect(globals, |c| {
            c.wait_for_layout();
            print_inspect_guide(c);
            wait_for_enter()?;
            let (inputs, registers) = c.inspect_summary();
            print_input_summary(&inputs, &registers);
            Ok(())
        }),
        ToolsCommand::ProfileBridge => profile_bridge::run(globals),
        ToolsCommand::Benchmark => run_command(globals, benchmark),
        ToolsCommand::Probe => probe::run(globals),
        ToolsCommand::Headless => headless::run(globals),
        ToolsCommand::RebootBootloader => run_command(globals, |c| {
            c.wait_for_layout();
            c.boot();
            Ok(())
        }),
    }
}

/// mirror the embedded `resources/` tree into the configuration home so the
/// bundled profiles' `executeCommand` scripts resolve on any system. existing
/// files are kept unless `--force`; `.sh` files are made executable.
fn run_init(force: bool) -> Result<()> {
    let home = sextant_core::resources::config_home();
    println!("init: mirroring resources into {}", home.display());
    let (mut written, mut skipped) = (0u32, 0u32);
    for rel in sextant_core::resources::resource_paths() {
        let Some(bytes) = sextant_core::resources::resource_bytes(&rel) else {
            continue;
        };
        let dest = home.join(&rel);
        if dest.exists() && !force {
            skipped += 1;
            continue;
        }
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&dest, &bytes).with_context(|| format!("writing {}", dest.display()))?;
        make_executable_if_script(&dest);
        println!("  {rel}");
        written += 1;
    }
    println!("init: {written} written, {skipped} kept (use --force to overwrite).");
    Ok(())
}

/// give an extracted `.sh` helper the executable bit (rust_embed carries content,
/// not permissions).
#[cfg(unix)]
fn make_executable_if_script(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if path.extension().and_then(|e| e.to_str()) != Some("sh") {
        return;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o755);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn make_executable_if_script(_path: &Path) {}

/// list the available profile sets (embedded plus any under the configuration
/// home) and the titles of the profiles each contains.
fn list_profiles() -> Result<()> {
    let sets = sextant_core::resources::available_profile_sets();
    if sets.is_empty() {
        println!("no profile sets found; run `sextant init` to extract the defaults.");
        return Ok(());
    }
    let width = sets.iter().map(String::len).max().unwrap_or(0);
    for name in &sets {
        let titles =
            match Configuration::from_sources(std::slice::from_ref(name), &[] as &[PathBuf]) {
                Ok(config) => config
                    .profiles
                    .iter()
                    .map(|p| p.title.clone())
                    .collect::<Vec<_>>()
                    .join(", "),
                Err(err) => format!("(failed to load: {err})"),
            };
        println!("{name:<width$}  {titles}");
    }
    Ok(())
}

/// list the connected palette / monogram devices and their type.
fn list_devices() -> Result<()> {
    let devices = sextant_usb::list_devices().map_err(common::usb_error)?;
    // real devices first, each with the leading `[N]` index `--device-index` selects.
    for (index, device) in devices.iter().enumerate() {
        println!(
            "[{}] {}  {:<12}  bus {} addr {}  {}",
            index,
            device.device_id,
            device.class.name(),
            device.bus,
            device.address,
            device.product.as_deref().unwrap_or("")
        );
    }
    if devices.is_empty() {
        println!("(no real palette / monogram devices found)");
    }
    // the in-memory fake is always available, selected by name rather than index.
    println!(
        "    {}  in-memory fake  (select with --device {})",
        common::SIMULATOR_DEVICE,
        common::SIMULATOR_DEVICE
    );
    Ok(())
}

/// open the gui for `connect --display`: the live attached device, or the fake
/// with `--device simulator`.
fn open_display(globals: &Globals) -> Result<()> {
    if globals.is_simulator() {
        run_display(globals, None)
    } else {
        sextant_sim::display::run_device(common::open_device_controller(globals)?, None);
        Ok(())
    }
}

/// open the gui and drive `step` each frame (a demo): the live device by default,
/// or the fake with `--device simulator`. the `&Controller<_>` the gui passes
/// coerces to `&dyn ControllerOps` here, where the trait is in scope.
fn display_demo(
    globals: &Globals,
    mut step: impl FnMut(&dyn ControllerOps) + 'static,
) -> Result<()> {
    if globals.is_simulator() {
        run_display(globals, Some(Box::new(move |c| step(c))))
    } else {
        let controller = common::open_device_controller(globals)?;
        sextant_sim::display::run_device(controller, Some(Box::new(move |c| step(c))));
        Ok(())
    }
}

/// launch the macroquad display over the in-memory fake device, optionally driven
/// each frame (e.g. by the demoscene). the live-hardware display is a separate
/// path (`run_device`); this one renders a built-in `--fake-layout`.
fn run_display(globals: &Globals, driver: Option<sextant_sim::display::FrameDriver>) -> Result<()> {
    let config = build_config(globals)?;
    let layout = layout_by_name(&globals.fake_layout)?;
    sextant_sim::display::run(config, layout, globals.echo, globals.debug, driver);
    Ok(())
}

/// print the connected layout and step-by-step capture instructions for the
/// `inspect` command. this goes to stderr so a redirect of stdout
/// (`sextant inspect > capture.txt`) keeps a clean log of just the input events.
fn print_inspect_guide(controller: &dyn ControllerOps) {
    eprintln!("inspect: exercise one control at a time and watch the [in] lines.\n");
    eprintln!("connected component(s):");
    for rc in controller.render_components() {
        eprintln!(
            "  i={} {} (kind {}) at slot {:?}",
            rc.index, rc.kind_name, rc.kind, rc.slot
        );
    }
    eprintln!(
        "\nmove each control deliberately and in isolation:\n\
         \x20 - buttons: press and release each one on its own\n\
         \x20 - dials / knobs: turn one detent at a time, each direction\n\
         \x20 - sliders: sweep slowly end to end\n\
         \x20 - joystick: push straight in each direction (left, right, up, down),\n\
         \x20   releasing back to center between pushes\n"
    );
    eprintln!(
        "every input prints one [in] line on stdout: the raw 8 registers, which\n\
         changed, their signed-8 reading, and the input types decoded from them.\n\
         a line ending in \"=> (unrecognized)\" is an input the decoder does not\n\
         model yet -- capture it (sextant inspect > capture.txt) and share it to\n\
         extend component support.\n"
    );
    eprintln!("press enter to stop.");
}

/// analyze a usbmon `.pcapng` offline: reassemble each direction's bulk stream,
/// decode the `0x7e`-framed msgpack length-aware, replay it through the same
/// analysis as the live path, and print the input and output summaries.
fn inspect_pcap(path: &Path) -> Result<()> {
    eprintln!("inspect (offline): {}", path.display());
    let mut device_to_host = Vec::new();
    let mut host_to_device = Vec::new();
    for (is_input, bytes) in pcap::read_bulk_payloads(path)? {
        if is_input {
            device_to_host.extend(bytes);
        } else {
            host_to_device.extend(bytes);
        }
    }
    let analysis = inspect::analyze_capture(
        &decode_msgpack_lenient(&device_to_host),
        &decode_msgpack_lenient(&host_to_device),
    );
    print_input_summary(&analysis.input_rows(), &analysis.register_rows());
    print_output_summary(&analysis.output_rows());
    Ok(())
}

/// print the host->device command summary -- the output protocol a capture
/// reveals (e.g. monogram led writes as `set_module i=<i> field=<f>`).
fn print_output_summary(outputs: &[InspectOutputRow]) {
    if outputs.is_empty() {
        return;
    }
    eprintln!("\noutput commands (host -> device):");
    for row in outputs {
        let range = match row.value_range {
            Some((lo, hi)) if lo == hi => format!("value {lo}"),
            Some((lo, hi)) => format!("value {lo}..{hi}"),
            None => String::new(),
        };
        eprintln!("    {:<26} x{:<6} {range}", row.signature, row.count);
    }
}

/// print the deduplicated input summary (to stderr, like the guide, so a stdout
/// capture stays a clean event log): per control the event count and value
/// range, plus each component's raw register activity -- which bytes carried
/// signal, which exposes channels no input type decodes yet.
fn print_input_summary(inputs: &[InspectInputRow], registers: &[InspectRegisterRow]) {
    if registers.is_empty() {
        eprintln!("\n=== inspect summary: no input events captured ===");
        return;
    }
    let total: usize = inputs.iter().map(|r| r.count).sum();
    eprintln!("\n=== inspect summary ({total} input event(s)) ===");
    for reg in registers {
        eprintln!("i={} {}", reg.index, reg.kind_name);
        for row in inputs.iter().filter(|r| r.index == reg.index) {
            let range = match row.value_range {
                Some((lo, hi)) if lo == hi => format!("value {lo}"),
                Some((lo, hi)) => format!("value {lo}..{hi}"),
                None => String::new(),
            };
            eprintln!("    {:<26} x{:<6} {range}", row.label, row.count);
        }
        let active: Vec<String> = reg
            .registers
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                r.map(|(lo, hi)| {
                    if lo == hi {
                        format!("v{i} {lo}")
                    } else {
                        format!("v{i} {lo}..{hi}")
                    }
                })
            })
            .collect();
        if !active.is_empty() {
            eprintln!("    registers (raw): {}", active.join(", "));
        }
    }
}

/// measure screen-string, single-led, and full-layout led throughput.
fn benchmark(controller: &dyn ControllerOps) -> Result<()> {
    controller.wait_for_layout();
    eprintln!("benchmark starting in two seconds.");
    std::thread::sleep(Duration::from_secs(2));

    let white = Led {
        i: 1,
        r: 255,
        g: 255,
        b: 255,
        m: 0,
    };
    let black = Led {
        i: 1,
        r: 0,
        g: 0,
        b: 0,
        m: 0,
    };

    let start = Instant::now();
    for _ in 0..1000 {
        controller.screen_string("ONE");
        controller.screen_string("TWO");
    }
    controller.wait();
    let screen_time = start.elapsed().as_secs_f64();

    let start = Instant::now();
    for _ in 0..1000 {
        controller.led(vec![white]);
        controller.led(vec![black]);
    }
    controller.wait();
    let led_time = start.elapsed().as_secs_f64();

    let start = Instant::now();
    for _ in 0..250 {
        controller.configure_leds(Some(255), Some(255), Some(255));
        controller.configure_leds(Some(0), Some(0), Some(0));
    }
    controller.wait();
    let configure_time = start.elapsed().as_secs_f64();

    controller.print_stats();
    println!("screen_string: {:.1} per second", 1000.0 / screen_time);
    println!("led: {:.1} per second", 1000.0 / led_time);
    println!("configure_leds: {:.1} per second", 250.0 / configure_time);
    Ok(())
}
