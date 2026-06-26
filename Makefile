# luaur fuzzing — AFL (cargo-afl) in two modes.
#
#   make setup-afl                 # one-time: install cargo-afl
#   make fuzz-typeck               # AFL real-time TUI (default target), runs til Ctrl+C
#   make fuzz-typeck SECS=300      # ...or stop this target after 300s
#   make fuzz-all SECS=300         # cycle ALL targets, 300s each
#   make fuzz-<target>             # AFL TUI for any of the 7 targets
#   LUAUR_FUZZ_ASAN=1 make fuzz-typeck   # AFL + AddressSanitizer build
#   make fuzz-triage-<target>      # minimize + bucket AFL crashes
#   make fuzz-standalone           # NO toolchain: deterministic cargo/CI fuzz
#
# Targets: compile run typeck number structured typeck_defs determinism

.PHONY: setup-afl fuzz-all \
	fuzz-compile fuzz-run fuzz-typeck fuzz-number fuzz-structured fuzz-typeck_defs fuzz-determinism \
	fuzz-triage-compile fuzz-triage-run fuzz-triage-typeck fuzz-triage-number fuzz-triage-structured fuzz-triage-typeck_defs fuzz-triage-determinism \
	fuzz-standalone \
	fuzz-standalone-compile fuzz-standalone-run fuzz-standalone-typeck fuzz-standalone-number fuzz-standalone-structured fuzz-standalone-typeck_defs fuzz-standalone-determinism

# ---------------------------------------------------------------------------
# AFL mode (real fuzzing; needs the AFL toolchain + nightly-free cargo-afl).
# ---------------------------------------------------------------------------

# Install the AFL runner (one-time).
setup-afl:
	cargo install cargo-afl

# Run a fuzz target under AFL's fork server + real-time TUI (requires setup-afl).
# Output (evolving corpus + crashes) lands in fuzz/artifacts/afl/<target>/.
# Set LUAUR_FUZZ_ASAN=1 for an AddressSanitizer build. By default a target fuzzes
# until you Ctrl+C; pass SECS=N to stop after N seconds, e.g. `make fuzz-typeck SECS=300`.
SECS ?=
fuzz-compile:
	cd fuzz && TARGET=compile FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-run:
	cd fuzz && TARGET=run FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-typeck:
	cd fuzz && TARGET=typeck FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-number:
	cd fuzz && TARGET=number FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-structured:
	cd fuzz && TARGET=structured FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-typeck_defs:
	cd fuzz && TARGET=typeck_defs FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-determinism:
	cd fuzz && TARGET=determinism FUZZ_SECS=$(SECS) ./scripts/run_afl.sh

# Cycle every target through AFL, giving each a time slice (default 300s — a
# bare `make fuzz-all` would otherwise sit on the first target forever).
# Override: `make fuzz-all SECS=600`.
fuzz-all:
	@secs="$(SECS)"; [ -n "$$secs" ] || secs=300; \
	for t in compile run typeck typeck_defs number structured determinism; do \
	  echo ">>> AFL $$t for $${secs}s"; \
	  ( cd fuzz && TARGET=$$t FUZZ_SECS=$$secs ./scripts/run_afl.sh ) || true; \
	done

# Minimize + bucket AFL crashes for a target.
fuzz-triage-compile:
	cd fuzz && TARGET=compile ./scripts/triage_afl_crashes.sh
fuzz-triage-run:
	cd fuzz && TARGET=run ./scripts/triage_afl_crashes.sh
fuzz-triage-typeck:
	cd fuzz && TARGET=typeck ./scripts/triage_afl_crashes.sh
fuzz-triage-number:
	cd fuzz && TARGET=number ./scripts/triage_afl_crashes.sh
fuzz-triage-structured:
	cd fuzz && TARGET=structured ./scripts/triage_afl_crashes.sh
fuzz-triage-typeck_defs:
	cd fuzz && TARGET=typeck_defs ./scripts/triage_afl_crashes.sh
fuzz-triage-determinism:
	cd fuzz && TARGET=determinism ./scripts/triage_afl_crashes.sh

# ---------------------------------------------------------------------------
# Standalone mode — NO AFL / nightly / sanitizer toolchain required. Builds the
# targets with --no-default-features and runs each over LUAUR_FUZZ_ITERS
# pseudo-random inputs (override: make fuzz-standalone ITERS=200000 SEED=1). Any
# panic / assertion break exits non-zero with a reproducible hex input. This is
# the deterministic, toolchain-free smoke fuzzer used in plain cargo / CI.
# ---------------------------------------------------------------------------
ITERS ?= 20000
SEED ?= 305419896

fuzz-standalone-compile:
	cd fuzz && cargo build --no-default-features --bin compile
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/compile
fuzz-standalone-run:
	cd fuzz && cargo build --no-default-features --bin run
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/run
fuzz-standalone-typeck:
	cd fuzz && cargo build --no-default-features --bin typeck
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/typeck
fuzz-standalone-number:
	cd fuzz && cargo build --no-default-features --bin number
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/number
fuzz-standalone-structured:
	cd fuzz && cargo build --no-default-features --bin structured
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/structured
fuzz-standalone-typeck_defs:
	cd fuzz && cargo build --no-default-features --bin typeck_defs
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/typeck_defs
fuzz-standalone-determinism:
	cd fuzz && cargo build --no-default-features --bin determinism
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/determinism

# Run every target's standalone smoke fuzz.
fuzz-standalone: fuzz-standalone-compile fuzz-standalone-run fuzz-standalone-typeck fuzz-standalone-number fuzz-standalone-structured fuzz-standalone-typeck_defs fuzz-standalone-determinism
