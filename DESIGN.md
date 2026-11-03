# design

architecture for the sextant host library and the avr firmware. this records
decisions and the target structure; modules are implemented phase by phase (see
[ROADMAP.md](ROADMAP.md)).

## overview

sextant is split into a host side (rust, std) and a firmware side (rust, no_std,
avr). the host enumerates a palette / monogram device over USB, runs a control
handshake, then exchanges framed messages over bulk endpoints. component input
arriving from the device is mapped through a behavior engine to actions; LED,
screen-string, and screen-image output flows back to the device. the same
controller runs unchanged against an in-memory fake device, which powers the gui
simulator and the test suite.

## workspace layout

the host is a cargo workspace; the firmware is a standalone avr crate excluded
from it (different toolchain and target).

```
crates/sextant-core   library: protocol, data model, behavior engine, controller
crates/sextant-fake   in-memory device emulator + simulated input + built-in layouts
crates/sextant-usb    real-device backend over nusb
crates/sextant-sim    macroquad gui simulator
crates/sextant-cli    the `sextant` binary (grouped subcommands; --device selects backend)
firmware/             avr firmware for the at90usb1286 (failsafe-first)
resources/            profiles (*.plp), scripts (*.sh), assets (*.raw/*.png), reference firmware
e2e/                  python (uv) black-box end-to-end tests
```

the `resources/firmware/*.hex` images and the palettegear `assets/.../reference/`
screen captures are proprietary reference data: embedded into local builds when
present, but gitignored so they are not redistributed in the upstream repo.

dependency direction: `cli -> {sim -> fake, usb, core}`; `fake -> core`;
`usb -> core`. `core` depends on neither the gui nor real usb.

## data model (sextant-core)

- **component**: a single module identified by a 3-char uid, with a *kind*
  (base, button, dial, slider, and the in-flight tri-dial / orbiter / core),
  an 8-slot value register, a screen string / icon, an LED color, and layout
  placement (slot, orientation, ports, parent/children).
- **layout**: the connected tree of components, built by traversing the device's
  reported `u/i/t/c` node format. it computes each component's cartesian slot and
  orientation, supports a query DSL (`[kind=Dial | rowwise | select(2)]`, bare
  uid, `[slot=(x,y)]`, ...), and exposes rowwise ordering and a bounding box.
- **behavior engine**: each input binding is a *behavior* (CYCLE, TRIGGER,
  ACCUMULATE, DELTA) plus an invocation that accumulates change, applies a value
  transform, and gates on a threshold. unlike the python original, which called a
  bound function pointer from inside the engine, the port has the invocation
  return a `DispatchValue` (a number, or a nested `(index, item)` for item
  cycles); the controller maps the action name to its effect. this keeps `core`
  free of controller state. the per-component stored value is threaded in by
  reference so bindings on one component share it, as upstream did via a dict.
  this is the most fidelity-sensitive logic and is locked by tests.
- **configuration**: parses paletteapp `.plp` json into profiles (title, icon,
  action map, media map, key map, layout, orientation), with the full USB HID
  usage table, the qt-modifier map, and the icon tables.

## configuration home and variable expansion

the embedded `resources/` tree is a *seed*, not a runtime source. the profile
sets' `executeCommand` bindings call helper scripts, and a binary cannot `exec` a
script it only carries as embedded bytes. rather than special-case scripts, the
whole tree is materialized once and everything is read back from there, so a
single editable location is the source of truth.

**the configuration home** is `$XDG_CONFIG_HOME/sextant` (falling back to
`$HOME/.config/sextant`). `sextant init` mirrors the embedded `resources/` tree
into it, preserving relative paths, so `<home>/profiles/`, `<home>/scripts/`,
`<home>/assets/`, ... line up 1:1 with the build. only what is embedded is
written: a clean release build carries just the public resources, while a local
build that has dropped in the proprietary firmware / reference blobs also mirrors
those into the (private) home -- which is the user's own machine, not the repo.
`init` is the only writer, and it never clobbers a file the user has edited (use
`--force` to overwrite). once seeded, the home is a normal editable tree the user
owns.

**runtime reads the home, never the embedded seed.** profiles (`--profiles
NAME` -> `<home>/profiles/NAME/`), the base screen images, the monogram logo, and
the font are all read from the home; `resources.rs` splits the two roles (embedded
`resource_*`/`resource_paths` for `init` only, `home_bytes`-backed `asset_bytes`
/ `monogram_*` / `user_profile_*` for runtime). this requires an explicit `init`:
device commands refuse with "run `sextant init`" until the home exists
(`is_initialized()` gates `build_config`), and `--profiles NAME` errors if that
set is not in the home. `sextant profiles` lists the sets found in the home.
explicit `--config PATH.plp` files are read directly and are unaffected.

**variable expansion** resolves `${NAME}` tokens in `executeCommand` argv at
dispatch time: `${SEXTANT_CONFIG}` expands to the configuration home, and any
other `${NAME}` to the process environment (empty if unset). so the bundled
profiles reference `${SEXTANT_CONFIG}/scripts/hello.sh` and find the script
regardless of the working directory. a program path that is still relative after
expansion is anchored to the home as a fallback, so a bare `./scripts/hello.sh`
also resolves. this is distinct from the upstream `%AMBIT_VALUE%` token, which
substitutes the live input value (and is also passed as the `AMBIT_VALUE` env
var); both are applied. a failed spawn is logged rather than silently swallowed,
so a missing script or unset `${NAME}` is visible.

## wire protocol

two formats, chosen by device class:

- **palettegear** (`16D0:09F8`, legacy) speaks minified json. messages are
  concatenated with no delimiter; a `screen_write` header is followed by a raw
  image blob that is split off on decode.
- **monogram CC** (`0483:A2C5`) speaks msgpack framed with `0x7E`: a single
  message is `7E <msgpack> 7E`; multiple are `7E m1 7E7E m2 7E`. (the framing
  splits naively on `0x7E`, which can also occur inside a payload -- this matches
  the original; a length-prefixed reframe is noted as a possible later fix.)

connection lifecycle: a six-step CDC control handshake (set/get line coding with
payload `E0 93 04 00 00 00 00 08`, i.e. 300000 baud, and set control line
state), then bulk transfers on the device's CDC data endpoints (discovered from
the descriptors -- palette revisions wire them onto different interfaces and
endpoint addresses, e.g. interface 1 IN `0x84` / OUT `0x05`). the host sends
`start`, clears existing maps, requests version and layout, then loops:
keepalive `check`, read input, write LED / screen / config updates. a 254->255
slider quirk is applied when `version_core < "1.3.2"` (string compare).

## device abstraction

`sextant-core` defines device traits (enumerate/open, control transfer, bulk
read/write, reset/release). `sextant-usb` implements them over real hardware via
nusb; `sextant-fake` implements them in memory. the controller is generic over
the trait, so nothing above the trait branches on real-vs-fake. this replaces the
python original's duck typing and its sed-generated `_simulator` script variants.

the cli selects the backend with `--device simulator` (vs a real
`VENDOR:PRODUCT`). so one command closure can run against either
`Controller<FakeDevice>` or `Controller<UsbDevice>`
without duplication, the cli erases the device type behind a small `ControllerOps`
trait it implements for `Controller<D>`; `run_command` builds the chosen backend,
connects, runs the closure, and closes.

the `sextant-usb` backend enumerates with nusb by `vendor:product`. a pure
`select_device` resolves which one to open from the recognized listing: `--device`
narrows to an id, `--device-index` picks the nth row (default the first), and it
returns the device's offset among ones sharing its id so the open-time `nth`
re-derivation lands on the same device. it then introspects the active
configuration to find the CDC data interface (its bulk in/out pair) and the
communications interface the line-coding handshake targets, detaches the kernel
cdc-acm driver, and claims the data interface. nusb's control transfers are
blocking (used directly for the handshake), but its bulk transfers are async,
while the controller's bulk i/o is blocking-with-timeout. each direction keeps a
persistent nusb queue; a transfer is submitted and the calling thread parks on a
custom waker until the transfer completes or the deadline passes. a timed-out read
is left in flight and reused on the next call so no device input is lost. full
validation needs physical hardware; enumeration and the not-found path are tested.

## controller threading

the controller owns worker threads coordinated by a shutdown flag:

- read: pull bulk input, decode, dispatch (`layout` / `input` / `version`).
- write: drain a write queue to bulk out.
- led: rate-limited LED frames (~60/s) with bounded-queue drop-on-overflow.
- screen-string: rate-limited screen text (~20/s), bounded queue. the monogram
  core has no device-side text, so the host rasterizes a 240x240 rgb565 frame and
  uploads it; `write_display` caches the last frame and re-sends only the 4-row
  strips that changed, so a text update over the static black background touches a few strips
  instead of all 60 (a failed strip drops the cache to force a full re-upload).
- keepalive: periodic `check`; re-send the screen string after idle.

queues use crossbeam channels (bounded for LED/screen, unbounded for writes) so
overflow policy and `len()` are available. input dispatch updates component value
registers, derives active input types, and runs the behavior engine, which
invokes callbacks (profile switch, set color, execute command, cycle mapping, and
the test behaviors).

all shared mutable state (config, layout, current colors, timers, counters) sits
behind one mutex held only briefly: a worker reads or mutates under the lock, then
releases it before any blocking queue send or sleep, so the rate-limited workers
are never starved and there is no lock re-entrancy. a layout update retains
component objects by uid so a component's stored cycle/accumulate state survives a
disconnect and reconnect; the connected count comes from the rebuilt index.

## simulator (sextant-sim)

a macroquad window renders the layout: component bodies, an LED outline in the
current color, id / slot / behavior labels, the base screen icon (decoded from
the `.raw` format) and its screen string, and raw value vectors. the keyboard maps
onto fake input: digit keys select a component, page up/down cycle selection,
arrows drive dial rotation and slider movement, space presses/releases buttons,
tab rotates the layout (re-traversing it with a new base orientation), and q / esc
quit.

the placement math is a pure `geometry` module (slot extrema -> per-slot size and
origin -> each component's body / inset-led / per-cell-connector / attachment-point
pixels) with no gpu
dependency, so it is unit tested against the exact pixel coordinates the upstream
pygame renderer produced. the key mapping is a pure `input` module, also tested.
only the macroquad frame loop (`display`) needs a window; it pulls a render
snapshot from the controller each frame, which replaces the original's fixed
60/30/5 hz tick scheduler, and is verified manually.

## screen-image codec

device screen icons use a trimmed 4bpp format: a 48-byte palette (16 bgr
entries) followed by 8192 body bytes (two 4-bit palette indices per byte, the
body stored in reverse byte order), 8240 bytes total. decode rebuilds a 128x128
image; encode quantizes a png to the 16 most-common colors (index 0 forced to
black) and packs nibbles. to match the committed reference `.raw` assets byte for
byte, the quantizer reproduces an upstream quirk: pygame built its comparison
colors from a packed integer read as rgba, which shifted every channel and
dropped blue, so nearest-color is measured on the red and green channels only.
json encoding likewise preserves object key insertion order (`serde_json`
`preserve_order`) so the wire bytes match the original python dict ordering.

the palette base imposes a hard constraint on uploading these: its core
watchdog-resets on a single bulk-out transfer >= ~3KB (it survives 2048B, resets
at 3072B; measured with the `probe` subcommand). the ~8KB `screen_write` is well
over that, and the firmware reassembles the bulk stream (so usb-level chunking
does not help) while the command has no offset field (so the image cannot be
split across messages). therefore image uploads are NOT part of connecting:
`configure_display` only selects a slot (`screen_display`), and the bitmap is
uploaded solely by the explicit `upload` command. every device write funnels
through one `bulk_transfer` chokepoint, which under `--debug` logs each transfer
(`[bulk] N bytes`) and each image upload (`[screen] upload ...`).

## headless mode and end-to-end tests

the `sextant tools headless` subcommand wires the controller to the fake device
and reads a line protocol on stdin (`pressed 6`, `rotation_left 2 8`, `slide_up 7
4`, `sleep 0.25`, `layout_changed prokit`, `configure_images`, `dump`, `quit`). a
`dump` writes one json line of device + controller state (screen string, screen
display, leds, per-component values / orientation / slot / flip, message counts,
screen images as hex, and the error / drop counters). the python e2e suite
(`e2e/`, run by `make test-e2e` under uv) drives this protocol as a black box and
asserts the upstream `test_ambit.py` behavioral contract: geometry, the four
behavior types, profile and cycle-mapping switching, byte-exact image upload,
layout changes, the slider workaround, and keepalive timing. the upstream timers
initialize to the epoch; the port keeps the write/led clocks current but starts
the screen clock in the past so the first keepalive tick still performs a screen
reset, preserving the observable keepalive cadence.

## firmware failsafe (the never-brick guarantee)

the factory atmel dfu bootloader lives in the ISP-only BOOT section and flashes
the application (USER) section. a broken application that never returns to dfu is
only recoverable over ISP. the failsafe makes the return path automatic:

- an eeprom boot token: `128` runs the firmware; any other value (including the
  `0x00` left by a dfu erase) jumps straight to the dfu bootloader.
- on boot, read the token; if it is not `128`, jump to dfu immediately. otherwise
  set the token to the bootloader value, bring the app up, and only commit it
  back to `128` after a health gate (usb enumerated + stable). any crash, reset,
  or bad flash before the commit lands back in dfu on the next boot.

this improves on the C original, where the commit step (`bootloader_deactivate`)
was never called. the failsafe needs no usb, so it is the first firmware
deliverable; usb-cdc, LED pwm, and the json command set follow. avr-rust uses the
avr-hal stack (embassy does not target 8-bit avr); a usb driver for the
at90usb1286 must be hand-written, as published crates cover only the 32u4.

the safety-critical *decision* (run vs. recover-to-dfu) is split out into the
dependency-free `no_std` `sextant-failsafe` crate so it is unit tested on the host
through the full boot lifecycle -- a bug there bricks hardware. the avr `main`
(`firmware/src/main.rs`) is thin glue: it reads the token over the svd-generated
`avr-device` pac, applies the decision, arms before running, drives an LED
power-on self test, enables a ~4 s watchdog (so a hang auto-recovers to dfu), runs
the health gate, and commits. it builds with nightly + `avr-none` + build-std +
avr-gcc into a flashable image; on-hardware verification of the never-brick
guarantee is mandatory before any production flash and is not possible without a
device.

## dependencies

host: serde / serde_json (json), rmpv (msgpack), clap (cli), crossbeam-channel
(queues), nusb (usb), macroquad (gui), rust-embed (resources), image (png decode),
rand (demos), thiserror / anyhow (errors). firmware: avr-hal / avr-device, ufmt.
e2e: python via uv.

## simplifications and quirks

- one `sextant` binary with grouped subcommands replaces 19 scripts; `--device
  simulator` replaces the duplicated `_simulator` files and their sed codegen.
- the wide-component 180-degree third female slot was a confirmed upstream typo
  (`(x-1, x+1)`); it is corrected to `(x-1, y+1)`, the 180-degree rotation of the
  0-degree case, and locked by an off-origin regression test (the upstream tests
  only sampled the origin, where `x == y` masked the bug).
- the python `console` REPL has no clean rust analog (likely a scripted mode or
  dropped); decided in phase 5.
- monogram CC support was incomplete upstream; the nusb backend and msgpack
  codec are now validated against a real monogram core module, and the new module
  input formats were reverse-engineered from hardware and implemented in the
  component model, locked by unit tests. each sub-control binds independently
  through an indexed input type (`pressed_2`, `rotation_left_3`, `joystick_x`):
  the core buttons and tri-dial presses decode as per-bit `pressed_n` edges, the
  tri-dial knobs as per-register `rotation_<dir>_n`, and the orbiter as a signed
  rotation plus two big-endian signed-16 joystick axes. the 0-based base index is
  also resolved. the one remaining gap (the orbiter's third joystick channel and
  press-framing center) is documented in
  [docs/monogram-protocol.md](docs/monogram-protocol.md). the `sextant inspect`
  command drives the next round of this work: it guides the user through
  exercising each control and prints a decoded line per input event (raw
  registers, what changed, the derived input types, `(neutral)` for the all-zero
  reset frame, and `(unrecognized)` for a nonzero unmodeled input), so a new
  module's format can be read off directly; the lower-level `sextant --debug`
  flag dumps every message both directions. the model above was validated on
  hardware through `inspect` (~2900 live events; every exercised control
  decoded), which is also how the all-zero reset frame was identified.
