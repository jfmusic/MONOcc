SHELL := /bin/bash

.DEFAULT_GOAL := build

CARGO ?= cargo
CLI := sextant-cli

# run the sextant binary (used by the asset-conversion target)
RUN := $(CARGO) run -q -p $(CLI) --

# embedded resources (layouts, assets, reference firmware) live with the core
# crate so they can be compiled into the binary.
RESOURCES := crates/sextant-core/resources

# dfu / flashing
DFU_MCU := at90usb1286
FIRMWARE_LATEST := 1.4.6136
FIRMWARE_LEDOPT := 1.3.1
ASSET_INDICES := 23 24 25

.PHONY: help
help: ## list available targets
	@grep -hE '^[a-zA-Z0-9_%-]+:.*?## ' $(MAKEFILE_LIST) \
		| sort | awk 'BEGIN{FS=":.*?## "}{printf "  %-22s %s\n", $$1, $$2}'

# --- build / test ---

.PHONY: build
build: ## build the host workspace (debug)
	$(CARGO) build

.PHONY: release
release: ## build the host workspace (release)
	$(CARGO) build --release

# static release binary: musl yields a portable, self-contained executable that
# the release workflow uploads as a github release asset.
MUSL_TARGET := x86_64-unknown-linux-musl
DIST_BIN := target/$(MUSL_TARGET)/release/sextant

.PHONY: dist
dist: ## build the static (musl) release binary
	$(CARGO) build --release --target $(MUSL_TARGET) -p $(CLI)
	@echo "dist: $(DIST_BIN)"

.PHONY: check
check: ## type-check the host workspace
	$(CARGO) check

.PHONY: test
test: ## run rust unit + integration tests
	$(CARGO) test

.PHONY: test-e2e
test-e2e: ## run python end-to-end tests (uv)
	@if [ -f e2e/pyproject.toml ]; then cd e2e && uv run pytest; \
	else echo "test-e2e: not yet implemented (see ROADMAP.md phase 3)"; fi

.PHONY: fmt
fmt: ## format the code
	$(CARGO) fmt

.PHONY: lint
lint: ## run clippy (warnings as errors)
	$(CARGO) clippy --all-targets -- -D warnings

.PHONY: precommit
precommit: ## fmt check + lint + test (run before committing)
	$(CARGO) fmt --check
	$(MAKE) lint
	$(MAKE) test

.PHONY: setup
setup: ## install toolchains (mise) and fetch dependencies
	mise install
	$(CARGO) fetch

.PHONY: clean
clean: ## remove build artifacts
	$(CARGO) clean
	rm -rf ./out

# the device, demo, console, map, upload, and reboot operations are now `sextant`
# subcommands (see `sextant --help`); they are no longer wrapped as make targets.

# --- assets ---

.PHONY: assets
assets: ## build device screen images (.raw) from png sources
	@for i in $(ASSET_INDICES); do \
		$(RUN) resource convert $(RESOURCES)/assets/$$i.png $(RESOURCES)/assets/$$i.raw; \
	done

# --- firmware (avr) ---

FIRMWARE_ELF := firmware/target/avr-none/release/sextant-firmware.elf

.PHONY: firmware
firmware: ## build the avr failsafe firmware image (nightly + avr target)
	cd firmware && cargo +nightly build --release
	@avr-size $(FIRMWARE_ELF)

.PHONY: firmware-hex
firmware-hex: firmware ## produce a flashable intel hex from the firmware elf
	avr-objcopy -O ihex -R .eeprom $(FIRMWARE_ELF) firmware/target/sextant-firmware.hex
	@echo "hex: firmware/target/sextant-firmware.hex"

.PHONY: firmware-test
firmware-test: ## run the host-testable never-brick decision logic
	$(CARGO) test -p sextant-failsafe

.PHONY: flash
flash: flash-reference-$(FIRMWARE_LATEST) ## flash the latest reference factory firmware

.PHONY: flash-ledopt
flash-ledopt: flash-reference-$(FIRMWARE_LEDOPT) ## flash the led-optimized reference firmware (1.3.1)

flash-reference-%: ## flash a specific reference firmware version via dfu-programmer
	dfu-programmer $(DFU_MCU) erase
	dfu-programmer $(DFU_MCU) flash $(RESOURCES)/firmware/firmware-$*.hex
	dfu-programmer $(DFU_MCU) launch
