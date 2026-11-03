# sextant

cross-platform control software and firmware for PaletteGear and MonogramCC

evolved from **ambit** and **ambit-firmware**, now beyond feature parity.

## what it is

palette / monogram devices are modular controllers: magnetically-connected
dials, buttons, sliders, and a base with a small screen, all daisy-chained over
an internal I2C bus and exposed to the host as a USB CDC serial device. sextant
talks that protocol from the host, maps component input to actions (HID keys,
MIDI notes, shell commands, profile switches, LED colors), parses paletteapp
`.plp` configuration, renders a device-free gui simulator, and drives LED demos.

sextant is primarily host software. an alternative AVR firmware for the device
base also lives here, but it is early and HIGHLY EXPERIMENTAL -- see
[firmware](#firmware-experimental).

## install

each tagged release attaches a static, self-contained linux x86-64 binary built
by the release workflow -- no runtime dependencies, just drop it on your PATH.
download the latest, verify its checksum, and install it:

```
base=https://github.com/khimaros/sextant/releases/latest/download
curl -LO $base/sextant-x86_64-linux-musl
curl -LO $base/sextant-x86_64-linux-musl.sha256
sha256sum -c sextant-x86_64-linux-musl.sha256
install -Dm755 sextant-x86_64-linux-musl ~/.local/bin/sextant
```

then run `sextant init` once to extract the default profiles and helper scripts
into `~/.config/sextant` (sextant reads them from there), and `sextant connect
--display --device simulator` to try the gui with no hardware. other platforms
build from source (below).

## usage

the binary is `sextant <subcommand>`; the top-level subcommands are `devices`,
`connect`, `profiles`, and `init`, plus four grouped families:

- `demo {sinebow,lavalamp,lightshow}` -- led demos (add `--display` to watch in
  the gui).
- `map {hid,midi}` -- program hid / midi mappings onto a device.
- `resource {convert,display,upload}` -- convert / display screen images, or
  upload them to a device (`upload N.raw N.png ...`, slot from the filename stem;
  `.png` converted; no args uploads the bundled base images).
- `tools {console,inspect,benchmark,probe,reboot-bootloader,headless}` --
  developer, debugging, and device-servicing tools (`headless` is the test-only
  scriptable mode).

`sextant devices` lists the connected palette / monogram devices and their type,
each with a leading `[N]` index, plus the always-available `simulator`; the
device-driving commands auto-detect the
first connected device, or take an explicit `--device VENDOR:PRODUCT` and/or
`--device-index N` to pick the nth row of that listing (`--device` narrows the
listing to that id first). `sextant profiles` lists the profile sets in the
configuration home (see below) selectable with `--profiles`.
anywhere a command talks to a device, `--device
simulator` (or a `_simulator` make variant) runs it against an in-memory fake
device instead, tuned by the `simulator options` flags (`--fake-layout`,
`--fake-firmware`). e.g.:

```
sextant --device simulator --fake-layout prokit demo sinebow
sextant resource convert in.png out.raw
```

### configuration home

sextant reads its profiles and screen assets from the configuration home,
`~/.config/sextant` (`$XDG_CONFIG_HOME/sextant` when set). the binary only
carries these as a seed; `sextant init` mirrors them into the home. it is a
required first step -- device commands refuse until it has run:

```
sextant init          # extract profiles + helper scripts + assets (keeps edits)
sextant init --force  # overwrite with the bundled versions
```

once seeded, edit `~/.config/sextant/profiles/<name>/*.plp` to customize a set;
`--profiles <name>` loads it from there and `sextant profiles` lists what is
present. the shipped `executeCommand` bindings reference
`${SEXTANT_CONFIG}/scripts/...`, which expands to the home regardless of where
you run `sextant`; inside an argv, `${SEXTANT_CONFIG}` is the home, `${NAME}` is
an environment variable, and `%AMBIT_VALUE%` is the live input value (also passed
as the `AMBIT_VALUE` env var).

`sextant connect --display` renders the live attached device; `sextant connect --display
--device simulator` instead drives a built-in `--fake-layout` on the in-memory
fake device (expertkit, prokit, base-only, monogram), e.g. `sextant connect --display
--device simulator --fake-layout monogram` shows the monogram core / tri-dial /
orbiter with no hardware. the monogram modules are
placed in their physical screen-upright arrangement (core vertical on the right,
orbiter upper-left, tri-dial below), with each module's per-cell magnetic
connectors and attachment point drawn and the base / core usb port marked; both
device families render this way. `Tab` rotates the whole setup,
including the live core screen. press `?` for an on-screen keybinding overlay.

the monogram core / tri-dial / orbiter are decoded so each sub-control binds on
its own: per-button presses, per-knob rotation, and the orbiter's joystick axes.
to learn or extend an unrecognized control, `sextant inspect` walks you through
exercising each one and prints a decoded line per input event (raw registers,
what changed, and the input types derived; `=> (unrecognized)` marks a nonzero
input nothing models yet, `=> (neutral)` the all-zero reset a delta control
sends after a move). `sextant inspect > capture.txt` saves a clean log to share.
on exit it prints a deduplicated summary: per control the event count and value
range, plus each component's raw register activity (which exposes channels no
input type decodes yet).

`sextant inspect <file.pcapng>` runs the same analysis offline on a usbmon
capture, decoding both directions: the input summary and the host->device
output commands (e.g. monogram led writes as `set_module i=<i> field=<f>`). the
lower-level `--debug` flag instead dumps every message both directions
(e.g. `sextant --debug run`).

## build from source

```
make            # build the host workspace (default target)
make help       # list every target
make test       # rust unit + integration tests
make test-e2e   # python end-to-end tests
```

## firmware (experimental)

> work in progress -- HIGHLY EXPERIMENTAL. the alternative AVR firmware for the
> at90usb1286 in the device base is under active development. it has NOT been
> verified on hardware, and most device features (usb-cdc, led pwm, the json
> command set) are not implemented yet. do not flash a device you care about.

the firmware is built failsafe-first: the foremost goal is a bulletproof
bootloader so a device can never be bricked during an update (R35). the
never-brick decision is host-tested, but on-hardware verification on a teensy++
2.0 / dev board is mandatory before any real flash. it is a separate nightly +
avr build (`make firmware`, `make firmware-hex`), excluded from the host
workspace.

see [firmware/README.md](firmware/README.md) for the build, the never-brick
design, and the verify-before-you-flash checklist; ROADMAP.md phase 7 tracks
status.

## documentation

- [REQUIREMENTS.md](REQUIREMENTS.md) -- the user-facing surface that must not regress
- [DESIGN.md](DESIGN.md) -- architecture, wire protocol, and the firmware failsafe
- [docs/monogram-protocol.md](docs/monogram-protocol.md) -- the monogram CC wire protocol and input model
- [ROADMAP.md](ROADMAP.md) -- phased plan and current status
- [CONTRIBUTING.md](CONTRIBUTING.md) -- build, test, and style workflow
- [firmware/README.md](firmware/README.md) -- the avr firmware and never-brick design

## origins

ported from the python `ambit` and C `ambit-firmware` projects. the port keeps
all user-facing functionality while simplifying the implementation (one binary
with subcommands instead of 19 scripts, trait-based device abstraction instead of
duck typing, no code-generated simulator variants).

## license

GPL-3.0-or-later, inherited from ambit. see [LICENSE](LICENSE).
