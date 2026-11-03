# contributing

## workflow

- before starting a task, add it to [ROADMAP.md](ROADMAP.md); mark it done when
  complete.
- acknowledge product requirements in [REQUIREMENTS.md](REQUIREMENTS.md); never
  regress a requirement in a release.
- update [DESIGN.md](DESIGN.md) after architectural changes and
  [README.md](README.md) after user-visible changes.
- propose style-only cleanups separately from functional changes.
- version control is the maintainer's job: do not commit, push, tag, or
  otherwise mutate git history.

## build and test

```
make build      # build the host workspace
make test       # rust unit + integration tests
make test-e2e   # python end-to-end tests (uv)
make precommit  # fmt check + clippy + tests; run before any commit
```

`make help` lists all targets. `make setup` installs the pinned toolchains via
mise and fetches dependencies.

## bug fixes

investigate, form a hypothesis, then write a failing test that reproduces it.
confirm the test fails before fixing.

## testing strategy

- prefer end-to-end integration tests over unit tests.
- the e2e suite is python (in `e2e/`, run with `make test-e2e`) and drives the
  compiled `sextant` binary as a black box, including a headless scriptable mode
  for the fake device.
- rust unit / integration tests are kept for pure-function fidelity where exact
  bytes or math matter (message codec, coordinates, behavior engine, image
  codec, query DSL) and for the controller-against-fake contract.

## code style

- ASCII only, in code, comments, docs, and output.
- documentation, comments, and command-line output are lowercase; CAPS only for
  acronyms or emphasis.
- comments explain *why*, not *what*; avoid changelog-style and obvious comments.
  remove dead code rather than commenting it out.
- define magic constants as named values at the top of the file that uses them,
  or in a shared module when used across files.
- keep it DRY; extend existing components rather than adding parallel ones.
- prefer pure functions; aim for functions under ~50 lines; reduce total line
  count without making the code cryptic.
- keep external dependencies minimal and justified; use mise for deterministic
  builds.

## project layout

see [DESIGN.md](DESIGN.md). in short: `sextant-core` is the gui-free, real-usb-free
library; `sextant-fake` and `sextant-usb` both implement the core device traits;
`sextant-sim` is the gui; `sextant-cli` is the binary; `firmware/` is the
standalone avr crate.

## adding a subcommand

add the variant to the clap command enum in `sextant-cli`, implement it against
`sextant-core` (selecting `sextant-usb` or `sextant-fake` via the global
`--device` flag, e.g. `--device simulator`). do not create separate `_simulator`
binaries or make targets -- the `--device` flag covers both backends.

## firmware

the avr firmware is built from `firmware/` with a nightly toolchain and the avr
target; it is excluded from the host workspace. see
[firmware/README.md](firmware/README.md). the failsafe bootloader is the first
deliverable and must be verified on a dev board before any production-device
flashing.

## research

read the source of the libraries in use to learn intended usage; fall back to
offline docs, then online docs, then prior knowledge. the upstream python
`ambit` and C `ambit-firmware` trees are the reference for behavior.
