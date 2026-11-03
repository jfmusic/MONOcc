# roadmap

the port proceeds host-first, then simulator and demos, then real usb, then
firmware (failsafe-first). nothing in a phase is marked done until its tests pass
and `make precommit` is clean. legend: `[x]` done, `[-]` in progress, `[ ]` todo.

## phase 0 -- scaffolding and docs

- [x] cargo workspace, crate skeletons, firmware crate (excluded)
- [x] mise + makefile + .gitignore + license
- [x] design / requirements / roadmap / contributing / readme

## phase 1 -- core model and codec (sextant-core) [x]

- [x] message codec: json + msgpack framing (byte-for-byte)
- [x] coordinates: male / female slot geometry (+ wide), with tests
- [x] component: kinds (incl. tri-dial / orbiter / core), value registers,
      behavior engine (cycle / trigger / accumulate / delta)
- [x] layout: traverse, slot / orientation, query DSL, rowwise, extrema
- [x] configuration: `.plp` parsing, HID + qt-modifier + icon tables
- [x] image: `.raw` decode + png->raw encode, locked to reference assets
- [x] resources: embed layouts + assets; flags: global config struct
- [x] reconcile flagged quirks (wide-180 coordinate) against tests

## phase 2 -- device abstraction, fake device, controller [x]

- [x] device traits (enumerate/open, control, bulk, reset/release)
- [x] sextant-fake: handshake phase machine, input injectors, built-in layouts
      (expertkit / prokit / base-only / monogram)
- [x] controller: worker threads + bounded/unbounded queues, keepalive,
      input dispatch, led / screen / image output, hid / midi / display config,
      version handling + slider workaround, callback set
- [x] rust integration tests against the fake device

## phase 3 -- python e2e harness (make test-e2e) [x]

- [x] headless scriptable mode (drive input events, dump device state as json)
- [x] port behavioral contract from test_ambit.py: keepalive, multifunction
      buttons, cycle-mapping color select, behaviors (cycle/trigger/accumulate/
      delta), configure_images (byte-exact), layout_changed, slider_range
      broken/fixed, default_input. (layout-query DSL is covered by rust unit
      tests; reference_meta's mediaMap-mixed path and execute-command subprocess
      callbacks are environment-dependent and deferred.)

## phase 4 -- simulator gui (sextant-sim, macroquad) [x]

- [x] display + component renderer: bodies, leds, labels, screen icon + string,
      values. (the macroquad frame loop subsumes the upstream 60/30/5 hz tick
      scheduler; it polls a render snapshot and redraws each frame.)
- [x] keyboard handling + event forwarding to fake input injectors
- [x] render-geometry contract test (pure geometry module reproduces the exact
      pygame pixels from AmbitSimulatorTest; the gpu render loop is not
      headless-testable, so it is verified manually via `sextant display --device simulator`)
- [x] built-in `monogram` layout (core / tri-dial / orbiter) selectable via
      `--device-layout monogram`, so the gui can render the monogram shape (it
      traverses to the right modules and slots, locked by an integration test).
- [x] render the *live attached* device: the renderer is now generic over the
      device, so `sextant gui` (without `--simulator`) connects to the real
      hardware and draws it each frame -- live input shows in the window, no
      injection (the device feeds its own input). `simulator` / `gui --simulator`
      keep the keyboard-driven fake. screen support for the core (logo + host-
      rasterized status text) also landed and is confirmed on hardware.
- [x] monogram gui geometry + window fitting: modules now carry a cell footprint
      (core 2x1, tri-dial 1x2, orbiter 2x2) and the layout re-places the reported
      linear chain into the physical 2d arrangement (core top, orbiter below-left,
      tri-dial below the buttons); the window resizes to the cell grid so there is
      no empty margin. unit-tested (slots, extrema, body rects); the anchors are
      explicit constants, easy to nudge against the live window if any module is
      off by a cell.
- [x] monogram screen-upright default + per-cell connectors: orientation 0 now
      matches the device's native pose (core vertical on the right, orbiter
      upper-left, tri-dial below); each module draws its real per-cell magnetic
      connectors (and the core usb-c port); the core screen is host-rendered and
      counter-rotated so `Tab` turns the physical screen with the gui, content
      staying upright. unit-tested (slots, extrema, connector counts, frame rotate).
- [-] keybinding overlay: `?` toggles an on-screen help panel listing the gui /
      simulator keys.

## phase 5 -- cli binary and demos (sextant-cli) [x]

- [x] clap subcommands: run, gui, simulator, image, map-hid, map-midi,
      upload, reboot-bootloader, benchmark, console. (console has no clean
      rust analog for the ipython repl, so it connects and points at `headless`
      for scripted control; gui renders the simulator until the phase 6 backend.)
- [x] demos: demoscene (sinebow + FREQ/PHASE/META via dial/button polling),
      lavalamp, lightshow. (demoscene's live control needs device input, so it is
      exercised on hardware in phase 6; under --simulator it shows the animation.)
- [x] `--simulator` flag wiring (one closure over a `ControllerOps` trait runs on
      either backend); makefile targets exercised
- [x] vendor resources (layouts, assets, reference firmware) into the embedded
      resources tree; the real-usb backend is a stub erroring until phase 6
- [x] `devices` command + device-profile consistency: `sextant devices` lists
      connected palette / monogram devices and their class. `DeviceClass` is now
      the single source of truth (`Device::class()`, `legacy()` derived from it),
      threaded through the controller; the tri-dial multi-led write is keyed by
      component kind, not a hardcoded index; the fake device now answers the
      monogram `get_layout` / `get_version` handshake so the msgpack path is
      testable.
- [x] monogram demo + gui adaptation (the demos / gui assumed palettegear's
      discrete dial/button modules): the demoscene reads buttons device-agnostic
      (the monogram core's v[0] bitmask) and dials from a new controller-side
      rotation accumulator (`Component::accumulated`, exposed in render) because
      the tri-dial knobs are delta-only and polling cannot read them; lavalamp
      now waits for the layout before snapshotting components (it animated
      nothing on the slow-connecting monogram); `gui` snapshots and renders the
      attached device's layout (live state rendering is still deferred, phase 4).

## phase 6 -- real usb backend (sextant-usb, nusb) [x]

- [x] enumeration, six-step control handshake, bulk in/out, reset/release.
      (nusb control transfers are blocking; bulk is async, so it is driven on a
      persistent queue parked on the calling thread with a timeout, and a
      timed-out read is left in flight and reused.)
- [x] validated against a physical **monogram core module** (`0483:a2c5`):
      enumeration, the cdc handshake, bulk in/out, and the **msgpack codec** all
      work end-to-end (`failed_writes: 0`), and the device's layout + 3000+ live
      input events decoded. this also drove the new-module work: the core /
      tri-dial / orbiter input formats were reverse-engineered from the device and
      implemented + tested, with each sub-control bound independently via indexed
      input types (per-button `pressed_n`, per-knob `rotation_<dir>_n`, orbiter
      `joystick_x` / `joystick_y`), and the base index was generalized (monogram
      is 0-based). see [docs/monogram-protocol.md](docs/monogram-protocol.md).
      (caveat: the device re-enumerates intermittently; output to the monogram
      base now targets the right component but was not visually confirmed.)
- [x] validated against a physical **palette gear** (`16d0:09f8`): its bulk data
      lives on a different interface / endpoints than the monogram (interface 1,
      IN `0x84` / OUT `0x05`), so the backend now discovers the cdc data interface
      and its bulk pair (plus the control interface) from the descriptors instead
      of hardcoding, and the device-driving commands auto-detect the connected
      device when `--device` is unset. connects with the real layout
      (base / button / dial / slider), `failed_writes: 0`.
- [x] `inspect` command: a guided capture that lists the connected components,
      walks the user through exercising each control, and prints a decoded line
      per input event (raw registers, what changed, signed reading, derived input
      types; `=> (unrecognized)` flags a nonzero unmodeled input, `=> (neutral)`
      the all-zero frame a delta control sends to reset after a move). this is the
      tool for learning or extending a component's input format.
- [x] validated the full monogram input model on hardware via `inspect`: the
      core buttons, all three tri-dial presses and per-knob rotations, and the
      orbiter rotation + joystick x/y all decode correctly across ~2900 live
      events. the only `(unrecognized)` lines were the all-zero reset frame (now
      relabeled `(neutral)`). the orbiter's third joystick channel (`v[5..7]`)
      remains undecoded; it needs a targeted single-axis capture.

## phase 8 -- inspect: protocol analysis (live + offline pcap) [x]

extend `inspect` from a live event stream into a protocol analyzer that
deduplicates inputs into a summary and reads offline captures, so the monogram
(and any future module) can be characterized from hardware or a `.pcapng`.

- [x] shared `InspectAnalysis` model: feed it decoded input events; it dedups
      per (component, input type) with event counts and value ranges, and tracks
      per-register raw activity (which bytes carry signal, min..max) to surface
      undecoded channels like the orbiter's third axis.
- [x] live summary: `inspect` accumulates into `InspectAnalysis` and prints the
      deduplicated summary when the session ends (stderr, so a stdout capture
      stays a clean event log).
- [x] pcap reader: a hand-rolled usbmon `.pcapng` reader (no new deps) splits
      bulk payloads by direction; a length-aware msgpack decoder reframes them
      (tolerating a `0x7e` byte inside a value, which the live naive splitter
      cannot); `inspect <file.pcapng>` replays them through the same analysis. a
      capture with no layout still yields the per-register behavior summary.
- [x] output decode: host->device commands are summarized by signature
      (`set_module i=<i> field=<f>`, `get_check`, `clear_action`, ...), which
      reproduced the monogram led format -- `set_module [i, field, 0x00rrggbb]`
      -- straight from the official-app captures.

## phase 9 -- hardware connect / ux refinements [-]

driven by real-device behavior: tighten the connect sequence and on-screen echo.

- [x] connect-time core reset (fixed): the palette base re-enumerated on connect
      because `configure_display` uploaded the ~8kb icon (`screen_write`) on every
      connect, and the core watchdog-resets on a single bulk-out transfer >= ~3KB.
      the `probe` command nailed it: survives 2048B, resets at 3072B; rate is
      irrelevant (240 tiny writes back-to-back were fine); a usb-split upload still
      resets (the firmware reassembles, so the ~8kb message is the limit and
      palette `screen_write` has no offset field). fix: `configure_display` now
      only selects the slot (`screen_display`); image uploads happen solely via the
      explicit `upload` command. validated: 3/3 no-echo connects, 1 layout, max
      transfer 366B. a fake-device test (`connect_transfers_stay_under_the_core_limit`)
      guards it. all writes funnel through `bulk_transfer` (logs `[bulk] N bytes`
      and `[screen] upload` under --debug); `--echo` is now off by default. the
      device persists uploaded images across power cycles (confirmed by replug), so
      `upload` once makes custom icons stick and connects just select them.
- [x] default base screen is a plain black background for both device types: the
      bundled palette screen images reduce to slot 23 (the black base, the default
      `icon::BLANK`), and the monogram core logo asset is now a plain black frame
      (vendored logo discarded). the brand icon (slot 0) and the unused 24/25
      extras were dropped.
- [x] unify the simulator connector rendering across both device families: every
      module draws its per-cell magnetic connectors (metallic dots), the base and
      core show a usb port on their top edge, and every module marks its attachment
      (parent-facing) point. previously palette showed only a single root-port dot.
      monogram modules carry no male-port slot (hardcoded cell geometry), so each
      snaps its attachment marker onto the per-cell connector facing the core;
      palette keeps the model's orientation-based male port.
- [x] fold the `display` command into a global `--display` flag and rename `run`
      to `connect`: `connect --display` opens the gui, and `demo <name> --display`
      watches a demo run on the connected device -- the live device by default (the
      live-device gui gained an optional per-frame driver), or the fake with
      `--device simulator`. lavalamp and lightshow gained per-frame step drivers
      (like the sinebow `Demoscene`) so all three demos work under `--display`.
      R6 reworded.
- [x] close the remaining profile-behavior gaps (R21/R19): (a) `togglePreview`
      toggles the base screen between the active profile's icon and a blank screen
      (an interpretation of the undocumented paletteapp action). (b) long / double /
      triple press are detected host-side from press timing (`press_events`,
      LONG_PRESS 500ms on release, MULTI_TAP 300ms) and dispatched additively to the
      raw pressed / released edges; treated as press-family in the behavior engine.
      button / dial single-press only (not the indexed core / tri-dial sub-buttons).
      (c) `mediaMap` bindings are folded into the component hidkeys and programmed
      onto the device hidmap (the host cannot emit hid; `PK_MEDIA_*` are consumer
      usages the device sends), mirroring the keyMap. unit + e2e tests cover each
      (an `e2e/fixtures/behaviors.plp` + a headless `map_hid` command).
- [x] debugging aids for profile bindings: `--debug` now logs every callback bind
      / skip (unknown action) / fire / miss (`[callback] ...`), and `--debug` (and
      `--echo`) are finally applied on the `--display` path (`open_device_controller`
      and the fake gui both dropped them, so `connect --display --debug` was silent).
- [x] `tabType` "Palette" maps to the brand icon (slot 0); `configure_images`
      uploads slot 0 again when the (gitignored) reference asset is present, so
      `resource upload` can restore the brand icon.
- [x] `resource upload [IMAGES]...` takes explicit image paths (each slot is the
      numeric filename stem, `.png` converted on the fly); no args uploads the
      bundled set. so `resource upload resources/images/*.raw` programs a directory.
      the upload logs each slot to the console and no longer `clear()`s the led /
      hid / midi maps first (no led "dance"), leaving device state untouched.
- [ ] orbiter settle latency: input still drains slowly after a hard press; the
      cause (device-side fixed cadence vs. single-buffered reads) needs more
      hardware measurement before deciding whether to pipeline the bulk reads.
- [x] `--device-index` selects across the recognized listing: the index was
      only an offset within a single VENDOR:PRODUCT, and auto-detect always picked
      the first id, so `--device-index 1` could never reach a second device of a
      different type (a monogram alongside a palette). fixed: a pure `select_device`
      picks the index-th entry of the `sextant devices` listing (filtered by
      `--device` when given) and returns the per-id offset `UsbDevice::open` needs;
      `sextant devices` now prints a `[N]` index so it is discoverable.
- [x] monogram screen redraws were a full 240x240 upload every time (60 strips /
      ~115kb / 60 blocking bulk writes per frame), so text echoes could not keep
      up with the 50ms cadence. the `write_display_slow` protocol takes an
      arbitrary bounding box, so `write_display` now caches the last uploaded
      frame and re-sends only the 4-row strips that changed (a status-text update
      over the static logo touches ~5 strips, an unchanged keepalive re-render 0);
      strip size stays at the known-safe 1920 bytes. a failed strip invalidates
      the cache so the next render is a full re-upload. confirmed on hardware: a
      connect+idle run dropped from 180 strips to 60, and the screen is markedly
      more responsive with no tearing -- the core's `invoke_display` commit keeps
      un-rewritten strips, so it is a single framebuffer.
- [x] monogram default screen background is black, matching the palette: the core
      screen composited the vendored logo on every profile, unlike the palette
      whose default `icon::BLANK` is a clean black base. `screen_frame` now starts
      from black and only composites the sextant logo for the brand icon slot 0
      (`--icon 0`), so the default is black and the logo is opt-in exactly as on
      the palette.
- [x] disambiguate the two "layout" meanings: the `--layouts` flag actually loads
      named *profile* sets (`.plp` action maps), not device layouts, which clashed
      with `--device-layout` (the physical component arrangement). renamed the
      profile-set surface to "profiles": `--layouts` -> `--profiles`,
      `resources/layouts/` -> `resources/profiles/`, `resources::layout_paths` ->
      `profile_paths`. "layout" now means only the physical device arrangement.
- [x] cli command-surface cleanup: grouped the sprawling flat command list under
      parent subcommands -- `demo {sinebow,lavalamp,lightshow}` (demoscene ->
      sinebow), `map {hid,midi}` (was map-hid / map-midi), `resource
      {convert,display,upload}` (was the `image` group plus `upload`), and `tools
      {console,inspect,benchmark,probe,headless,reboot-bootloader}`. trimmed the
      over-long `inspect` short help. added a `profiles` command that lists the
      embedded profile sets and their titles.
- [x] make the bundled default profiles work on any system. they pointed their
      `executeCommand` bindings at `./example/scripts/*.sh` -- a cwd-relative path
      to scripts that were never ported, so the bindings were dead on every
      machine (including a repo checkout). fixed with a configuration home the
      profiles anchor to. design in DESIGN.md ("configuration home and variable
      expansion"); R37, and R20/R21 amended. sub-tasks:
  - [x] `<home>` resolver: `$XDG_CONFIG_HOME/sextant` else `$HOME/.config/sextant`
        (std::env only, no new dependency).
  - [x] port the missing helper scripts (`hello.sh`, `temperature.sh`,
        `unlock.sh`) as portable, harmless demos and embed them under
        `resources/scripts/`.
  - [x] `${NAME}` expansion in `executeCommand` argv: `${SEXTANT_CONFIG}` -> home,
        other `${NAME}` -> process env; a still-relative program path is anchored
        to the home. rewrote the bundled profiles to `${SEXTANT_CONFIG}/scripts/...`.
        a failed spawn is logged instead of swallowed.
  - [x] `sextant init [--force]`: mirror the whole embedded `resources/` tree
        into the home, preserving relative paths, never clobbering an edited file
        without `--force`.
  - [x] `--profiles NAME` reads `<home>/profiles/NAME/`; `sextant profiles` lists
        the home sets.
  - [x] runtime reads only the extracted home, never the embedded seed: profiles,
        base images, monogram logo / font all resolve under `<home>` (embedded
        `resource_*` is seed-only, used by `init`). require explicit init -- device
        commands refuse via `is_initialized()` on `build_config` and `--profiles`
        errors on an absent set.
  - [x] e2e coverage (init extracts + is idempotent; a `${SEXTANT_CONFIG}` script
        binding actually fires; a device command refuses on an un-initialized home)
        and README/usage docs.
- [x] unify backend selection on `--device`: dropped the `--simulator` boolean and
      the `simulator` subcommand; the in-memory fake is now `--device simulator`
      (so `gui --device simulator` is the fake gui). the two fake-only flags
      `--device-layout` / `--version-core` moved under a `simulator options` help
      heading so they read as options of the simulator device. `sextant devices`
      now also lists the always-available `simulator` so the selector is
      discoverable. renamed the `gui` command to `display`. dropped the `random`
      fake layout (and the `rand` dependency it pulled into `sextant-fake`), and
      renamed `--device-layout` -> `--fake-layout` and `--version-core` ->
      `--fake-firmware`.

## phase 7 -- firmware: bulletproof bootloader (avr-hal), failsafe-first [-]

- [x] avr toolchain + target spec; minimal no_std boot. (nightly + `avr-none` +
      build-std + avr-gcc; `make firmware` builds a ~500-byte at90usb1286 image,
      `make firmware-hex` emits a flashable intel hex.)
- [x] failsafe: eeprom boot token, conditional dfu jump, health-gated commit.
      (the never-brick *decision* is the dependency-free `sextant-failsafe` crate,
      host-tested through the full boot lifecycle; the avr `main` is the thin glue
      that reads the token, jumps to the factory dfu bootloader on anything but a
      committed run token, arms before running, and commits only after a health
      gate -- fixing the upstream c, which armed but never committed.)
- [x] LED POST indication, watchdog, port config
- [ ] verify never-brick on a dev board before any production flashing (needs a
      teensy++ 2.0 / palette base -- cannot run here; mandatory before any
      production flash per R35)
- [ ] (later) usb-cdc via usb-device + hand-written at90usb1286 driver
- [ ] (later) LED pwm, json command set

## release / packaging

- [x] `make dist` builds the static (musl) `sextant` binary; the `release`
      github workflow (`.github/workflows/release.yml`) attaches it plus a
      sha256 checksum to a github release on each `v*` tag push. README documents
      installing the prebuilt binary and flags the firmware as experimental.
- [x] keep proprietary reference data (factory firmware hex, palettegear
      `reference/` screen captures) out of the upstream repo: gitignored and
      untracked, still embedded into local builds when present.
- [x] retire the make wrappers that merely invoked `sextant` subcommands (device,
      demo, console, map, upload, reboot); R33 amended to match. build/test,
      assets, and firmware/flash targets stay.

## carried over from upstream (context)

ambit-firmware roadmap status at import: failsafe jump, basic LED control, LED
pwm, USB CDC control + bulk, json scaffolding, and the `boot` command worked; the
`start` / `check` / `led` commands were partial; serial debug, I2C component
comms, and display control were unstarted. ambit had in-flight, incomplete
monogram CC support (msgpack dispatch, new component kinds).
