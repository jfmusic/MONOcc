# requirements

the user-facing surface sextant must provide. these are the product requirements
ported from ambit / ambit-firmware. it is never okay to regress on a requirement
in a release. items not yet implemented are marked; tracking is in
[ROADMAP.md](ROADMAP.md).

## devices and protocol

- R1 support palettegear devices (`16D0:09F8`) over the legacy json protocol.
- R2 support monogram CC devices (`0483:A2C5`) over the msgpack protocol
  (carried as far as upstream implements it; remainder is roadmap).
- R3 perform the USB CDC control handshake and bulk message exchange, including
  keepalive, layout request, version query, and the `version < 1.3.2` slider fix.
- R4 select device by `--device VENDOR:PRODUCT` and `--device-index N`.

## command surface (single `sextant` binary, subcommands)

- R5 `connect` -- connect to a device and run the main input/keepalive loop.
- R6 `--display` -- open the graphical interface on the connected device (the live
  device by default, or the fake with `--device simulator`): `connect --display`,
  or `demo <name> --display` to watch a demo run on it.
- R7 `simulator` -- run the full controller against the in-memory fake device.
- R8 `demoscene` -- interactive sinebow LED demo with FREQ / PHASE / META dial
  control modes and switchable presets.
- R9 `lavalamp` -- drifting pastel per-component LED demo.
- R10 `lightshow` -- random single-component LED flashes.
- R11 `console` -- interactive session against a device (form TBD: scripted mode
  or interactive shell; the python IPython REPL has no direct rust analog).
- R12 `image convert <src.png> <dst.raw>` and `image display <file.raw>`.
- R13 `map-hid` -- program HID keyboard mappings from a config and exit.
- R14 `map-midi` -- program MIDI note mappings and exit.
- R15 `upload` -- upload screen images to the device base.
- R16 `reboot-bootloader` -- reboot a device into the DFU bootloader.
- R17 `benchmark` -- measure LED / screen throughput.
- R18 every device subcommand has a `--simulator` mode that uses the fake device.

## configuration

- R19 parse paletteapp `.plp` json: title, icon (`tabType`), action map, media
  map, key map, layout, orientation.
- R20 support multiple profiles loaded together and switching between them via
  configured bindings (`--profiles` named sets and explicit config paths). named
  sets and screen assets are read from the configuration home, which `sextant
  init` seeds; device commands require it and point the user at `init` when it is
  absent.
- R21 actions: previous/next/switch profile, set color (r/g/b), execute command
  (with `%AMBIT_VALUE%` substitution and `AMBIT_VALUE` env, plus `${SEXTANT_CONFIG}`
  and `${NAME}` variable expansion in argv), cycle mapping, toggle preview, and
  the test behaviors (cycle/accumulate/delta/trigger).
- R22 component behaviors: CYCLE, TRIGGER, ACCUMULATE, DELTA with threshold,
  limits, loop, invert, discrete, and per-binding overrides.
- R23 layout query DSL: bare uid, and bracketed `kind` / `index` / `slot` /
  `orientation` matchers with `rowwise` and `select(n)` filters.
- R37 `sextant init` mirrors the embedded `resources/` tree (profiles, scripts,
  assets) into the configuration home (`$XDG_CONFIG_HOME/sextant`, else
  `$HOME/.config/sextant`), preserving relative paths, so the shipped
  `executeCommand` bindings resolve on any system; it never overwrites a
  user-edited file without `--force`. the embedded tree is only a seed: at runtime
  profiles and screen assets are read from the home, not the binary.

## input mapping

- R24 map component input to USB HID keys (full HID usage table, qt modifiers,
  dial sensitivity / repeat).
- R25 automatic MIDI note binding for attached components.

## leds and screen

- R26 set per-component LED colors; rate-limited with graceful overflow drop.
- R27 set the base screen string and screen icon; screen reset / keepalive.
- R28 push 128x128 screen images in the device's trimmed 4bpp `.raw` format,
  with png-to-raw conversion.

## simulator gui

- R29 device-free window rendering the component layout, LEDs, labels, base
  screen icon + string, and live values.
- R30 keyboard control: digit keys select a component, page up/down cycle
  selection, arrows drive dial rotation and slider movement, space presses
  buttons, tab rotates the layout 90 degrees, q / esc quit.

## build, test, and tooling

- R31 `make` builds; `make precommit` runs fmt + lint + tests before committing.
- R32 `make test` (rust) and `make test-e2e` (python) green.
- R33 the device, demo, console, map, upload, and reboot operations are exposed
  as `sextant` subcommands (run against hardware or `--device simulator`); make
  retains the build/test/tooling targets (build, release, dist, check, test,
  test-e2e, fmt, lint, precommit, setup, clean, assets) and the firmware/flash
  targets (firmware[-hex/-test], flash[-ledopt], flash-reference-%).
- R34 run on linux, including headless for the non-gui paths.

## firmware

- R35 bulletproof failsafe bootloader: a device can never be bricked during a
  firmware update; any failed or interrupted update returns to the DFU
  bootloader automatically (ISP recovery is the documented last resort).
- R36 (later) USB CDC control, LED PWM, and the json command set on the
  at90usb1286, as the firmware matures past the failsafe.
