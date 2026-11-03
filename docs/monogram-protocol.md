# monogram cc protocol notes

what the sextant host learned by exercising a real **monogram core module**
(`0483:a2c5`) over usb. upstream `ambit` left the monogram (msgpack) support
incomplete and undocumented; this captures the wire details so the new module
kinds can be finished. everything here was observed live with `sextant --debug
run`, which dumps every message both directions. for a guided, decoded capture
of a single control, use `sextant inspect` (it lists the components, tells you
what to exercise, and prints one decoded line per input event, flagging anything
it cannot yet decode).

## transport

- composite usb device: cdc comms (interface 0), **cdc data (interface 1, with
  the bulk endpoints `0x01` out / `0x81` in)**, hid (interface 2), and audio
  (interfaces 3-4). the kernel binds `cdc_acm` to interfaces 0 and 1.
- the six-step cdc control handshake and bulk in/out work as ported; the msgpack
  framing (`0x7e` delimited) decodes correctly.
- the device **re-enumerates frequently** (its bus address changes and it
  disappears for stretches), so `nusb::list_devices()` intermittently misses it.
  open by `vendor:product` and retry. permissions are a non-issue if the udev
  rule grants your group access to the raw usb node.

## message types (device -> host)

| key | meaning | handled? |
|---|---|---|
| `l` | the component layout tree (sent as `{"l": <tree>, "l_time": <ts>}`) | yes |
| `l_time` | a standalone layout-clock timestamp (`{"l_time": 1836}`) | yes -- ignored heartbeat |
| `in` | a real input event | yes |
| `in_noop` | idle heartbeat (all-zero input) | yes -- ignored |
| `uptime` | device info: uptime, `version_core`, build, mcu uid, ... | yes -- `version_core` is extracted |

the layout message carries `l_time` as a second key, and the device's firmware
version arrives inside the `uptime` message rather than a standalone
`version_core` message; both are handled by the existing dispatch.

input arrives as `{"in": [{"i": <index>, "v": [8 bytes]}, null, null, null]}` --
a fixed four-slot array whose trailing nulls are skipped. **value deltas are
signed bytes**: `1`/`2` mean `+1`/`+2`, `255`/`254` mean `-1`/`-2`.

a delta control (rotation, joystick) emits a real `in` event with an **all-zero
`v`** right after a move, to reset its registers to neutral. this is distinct
from the `in_noop` idle heartbeat: it is a normal input event that decodes to no
input type, so the engine harmlessly ignores it. `inspect` labels it `(neutral)`
rather than `(unrecognized)` so genuine gaps stand out.

unlike the palettegear json layout, monogram component **indices are 0-based**
(the base/core is index 0, not 1).

## modules (this device)

a core (base, with the screen) plus a tri-dial and an orbiter. a separate
two-button module is physically present but is **not its own layout node** -- its
presses report through the core. only three components appear in the layout:

| module | index / kind | rotation | press |
|---|---|---|---|
| **core** | 0 / 25 | -- | `v[0]` bitmask: `1` = button a, `2` = button b, `3` = both |
| **tri-dial** | 1 / 9 | three knobs, one each on `v[1]`, `v[2]`, `v[3]` (signed delta) | `v[0]` bitmask: `1`/`2`/`4` per dial |
| **orbiter** | 2 / 24 | signed delta on `v[0]` | "press" is a **joystick**: two big-endian signed-16 axes, x = `v[1..2]`, y = `v[3..4]` (rest = `0`); a third channel and a `125, 94` center streamed in a distinct `},0,...]` framing are not yet modeled |

## output protocol (host -> device)

led and module config are written with `{"set_module": [index, field, value]}`,
**not** the palettegear `{"led": [...]}` message. this was read straight from the
official-app captures via `sextant inspect <capture.pcapng>`:

| command | meaning |
|---|---|
| `set_module [i, 2, 0x00rrggbb]` | set a single-led module's color (core, orbiter) |
| `set_module [i, 5, 0xssrrggbb]` | set a tri-dial sub-led: high byte `ss` = sub-led index (`0`/`1`/`2`), low 24 bits = rgb |
| `set_module [i, 178/180/181, n]` | orbiter analog config (non-led) |
| `set_module [i, 177, i]` | module identify / slot tag (sent at startup) |
| `get_check` | status / keepalive query |
| `clear_action [i]` | clear a module's action map (sent before a profile apply) |
| `get_layout` / `get_state` / `get_version` | query the device tree / state / firmware |
| `flip` | re-read the layout after a module is added / removed |

the color is packed **`0x00rrggbb`** (red is the high byte). ground truth: the
app sets a tri-dial sub-led to red = `0x01ff0000` and green = `0x0200fe00`. the
sextant `led_messages` already emits `set_module`, and now packs the bytes in
this order (an earlier version had red/blue swapped).

### core display (240x240 rgb565)

the core screen is **not** driven by the palettegear `screen_string` /
`screen_write` / `screen_display` messages -- that is why sextant leaves it
blank. the official app uploads a full bitmap with `write_display_slow` then
commits with `invoke_display`:

- `write_display_slow [more, x0, y0, x1, y1, <bytes>]` -- a rectangle from
  `(x0, y0)` to `(x1, y1)`. observed from the official app: `x0=0`, `x1=240`, 4
  rows per strip, 60 strips to cover the **240x240** screen. each strip is
  `240 * 4 * 2 = 1920` bytes, i.e. **16-bit rgb565** pixels (2 bytes each). the
  bounding box is arbitrary, so the host re-sends only the strips that changed
  since the last frame (see below) rather than a full 60-strip upload each time.
- `invoke_display <n>` -- commit / refresh the uploaded frame (`3` then `4`). the
  core keeps un-rewritten strips in its buffer (a single framebuffer), so
  committing after a partial write still refreshes a complete frame with no
  tearing -- confirmed on hardware, which is what makes the partial-upload path
  above safe.

so monogram screen support means rendering host-side to a 240x240 rgb565 buffer
and streaming it in 4-row `write_display_slow` strips -- a real feature (the
palettegear path sends text for the *device* firmware to render; the monogram
host must rasterize the pixels itself). **implemented and confirmed on
hardware**: the controller rasterizes the status text (8x8 font) over a 240x240
buffer and uploads it; big-endian rgb565 is confirmed by reconstructing a
captured frame. the background is plain black: the brand-icon slot (`--icon 0`)
composites the `monogram_logo()` base, which is now a black frame, so the screen
is black either way. the screen font is vendored under `assets/monogram/`.

## indexed input types

the multi-control modules (core, tri-dial) expose each sub-control through its
own input type so a `.plp` config can bind them independently. an input type
carries an optional trailing `_<n>` index that `split_index` peels off; the
behavior engine runs the binding under its base type, while the index selects
which sub-control (and which value register) it addresses:

| input type | module / control |
|---|---|
| `pressed_1` / `pressed_2` | core button a / b (also `released_n`) |
| `pressed_1` / `_2` / `_3` | tri-dial knob 1 / 2 / 3 press |
| `rotation_left_1` / `_2` / `_3` (and `rotation_right_n`) | tri-dial knob 1 / 2 / 3 turn |
| `set_1` / `_2` / `_3` | tri-dial knob 1 / 2 / 3 integrated absolute position |
| `set` (on the orbiter) | orbiter wheel integrated absolute position |
| `joystick_x` / `joystick_y` | orbiter joystick axes |

the tri-dial knobs and orbiter wheel are **delta-only** (the device never reports
an absolute position the way a palettegear dial reports `set` in `v[3]`). so the
host integrates each rotation delta into a wrapping 0-255 position
(`Component::accumulate`, with a `previous_accumulated` for the change) and emits
it as a `set` input alongside the rotation. a knob can therefore drive an
absolute `set`-style binding (or a delta `rotation_*` binding), matching a
palettegear dial.

## what the port implements

`Component::determine_input_types` / `determine_vindex` / `dominant_value`
(`sextant-core/src/component.rs`) decode the device, locked by unit tests built
from these captures and validated on hardware via `sextant inspect` (~2900 live
events; every control below decoded correctly):

- **core**: `v[0]` bitmask -> per-button `pressed_n` / `released_n` via per-bit
  edge detection, so each button binds on its own.
- **orbiter**: signed `v[0]` -> rotation left / right (sign-extended); the
  joystick's x / y -> `joystick_x` / `joystick_y`, each a big-endian signed-16
  axis decoded by `signed_axis`. rotation and joystick are independent and can
  fire in the same frame.
- **tri-dial**: `v[0]` bitmask -> per-dial `pressed_n`; each knob's signed delta
  on `v[1..3]` -> indexed `rotation_left_n` / `rotation_right_n`, with
  `determine_vindex` pointing at that knob's register.

## still open (needs a richer model)

- **orbiter third channel**: beyond x / y, the joystick streams more in
  `v[5..7]`. analyzing the official-app `orbiter-pad` capture (`sextant inspect
  monogramcc-orbiter-pad-up-right-down-left.pcapng`) confirms x = `v[1..2]` and
  y = `v[3..4]`, but `v[5..6]` behaves like a **saturating push-magnitude** (it
  climbs with the deflection then caps around `127`, after which `v[5]` engages)
  rather than a clean signed axis, so it is left unmodeled rather than guessed.
  a separate `},0,...]` framing also carries an absolute pair near `125, 94`.
  inspect's per-register summary surfaces these as active bytes; a layout-bearing
  single-axis capture would be needed to model them properly.

## host-assigned 2d layout

the device reports its modules as a linear chain (core -> tri-dial -> orbiter),
not a 2d arrangement, so the host places them (`monogram_cells` in
`component.rs`). orientation 0 is the **screen-upright** pose -- how the modules
sit when the core screen reads upright: the core is vertical on the right (screen
above buttons), the orbiter is the 2x2 to its upper-left, and the tri-dial is the
2x1 below the orbiter. `apply_monogram_geometry` rotates the whole arrangement by
the layout orientation, and `render_screen` counter-rotates the uploaded 240x240
frame (by `-orientation`) so the content stays upright in the user's frame as the
device physically turns with the setup -- a `Tab` rotate turns both the gui layout
and the physical core screen together.

## resolved

- **0-based base index**: the layout now records the root component's index
  (`ComponentLayout::base_index`), and the controller's screen output targets that
  base (`base()` / `base_mut()`) instead of a hardcoded index 1 -- so screen
  string / icon now reach the monogram core (index 0) as well as the palettegear
  base (index 1). led output already addresses each component by its own index.

## three-slider modules (kind 10)

The extracted Legacy 4.1 service defines `slider3x` with type ID 10.
Its actuation method forwards `v[1]`, `v[2]`, and `v[3]` to independent
absolute sliders. Local captures from 2026-11-02 confirm changes on these
registers for module indices 2 and 3. The decoder exposes these as `set_1`,
`set_2`, and `set_3`, preserving unsigned 0-255 values and emitting changed
channels only. Physical geometry, LEDs, and the meaning of occasional v[4]
data remain unverified for these modules.

## byte escaping (confirmed from official CoreV2 codec)

Before framing MessagePack, escape payload bytes 0x7d and 0x7e as 0x7d
followed by the byte XOR 0x20. Undo escaping after splitting wire frames.
The November 2 14:51 BBC test captured 935 slider events and exposed this:
center value 126 appeared as registers 125,94 and shifted the next channel.
Encoding, display-strip encoding, live decoding, and offline decoding now
handle escaping. A wire fixture verifies center=126 with neighbors=255.
All 60 core tests pass; live confirmation of the correction is still needed.

## three-button modules and layout refresh

The original Legacy 4.1 service defines `button3x` as kind 8: v[0] bits
0, 1, 2 map to its three buttons. The decoder now exposes indexed press and
release edges and uses the three-sub-LED command path with its commit.
The official app requests get_layout when l_time differs from its saved
layout clock; the controller now follows this behavior for hot swapping.
These fixes do not establish why the three physical button units were absent
from the initial reported layout. A debug capture is still needed.
