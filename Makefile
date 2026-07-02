# luaur fuzzing — AFL (cargo-afl) in two modes.
#
#   make setup-afl                 # one-time: install cargo-afl
#   make fuzz-typeck               # AFL real-time TUI (default target), runs til Ctrl+C
#   make fuzz-typeck SECS=300      # ...or stop this target after 300s
#   make fuzz-all SECS=300         # cycle ALL targets, 300s each
#   make fuzz-loop SECS=300        # cycle ALL targets, 300s each, FOREVER (Ctrl+C)
#   make fuzz-<target>             # AFL TUI for any target (see the list below)
#   LUAUR_FUZZ_ASAN=1 make fuzz-typeck   # AFL + AddressSanitizer build
#   make fuzz-triage-<target>      # minimize + bucket AFL crashes
#   make fuzz-standalone           # NO toolchain: deterministic cargo/CI fuzz
#   make fetch-corpus              # seed corpus with real Luau (conformance + upstream)
#
#   Throughput knob — LUAUR_FUZZ_STEPS caps the VM interrupt budget for the
#   run/splice/structured targets (default 100k). ~93% of those targets' time is
#   the VM loop, dominated by infinite-loop inputs running to the cap (they add no
#   new coverage), so a lower cap is a near-pure speedup:
#     LUAUR_FUZZ_STEPS=20000  make fuzz-loop   # breadth: ~6x more execs/sec
#     LUAUR_FUZZ_STEPS=1000000 make fuzz-run   # depth: exercise long finite loops
#
#   FUZZ_MAXLEN caps AFL input size (-G) for the DIRECT-SOURCE targets
#   (compile/run/typeck), default 8192. AFL's default is 1MB, so it grew ~14KB
#   inputs that each take 6-10ms to parse+check while the median input is <500B —
#   capping ~quadruples typeck check throughput at near-zero coverage cost.
#     FUZZ_MAXLEN=4096 make fuzz-typeck   # tighter (~6x), FUZZ_MAXLEN=0 disables
#
# Targets (bug-priority order): run typeck_typed typeck determinism compile structured splice optdiff metamorphic roundtrip spans number typeck_defs api gcstress host serde_roundtrip
#
#   Runtime/host/GC/serde targets:
#     make fuzz-api               # stdlib builtins + metamethods (string.pack/patterns/buffer/sort)
#     make fuzz-gcstress          # aggressive GC cadence over alloc-heavy programs
#     make fuzz-host              # Rust<->Lua embedding boundary (callbacks/userdata/registry)
#     make fuzz-serde_roundtrip   # serde to_value/from_value round-trip identity
#   Their toolchain-free smoke mode: make fuzz-standalone-api (etc.)

.PHONY: setup-afl fuzz-all fuzz-loop build-wasm fuzz-wasm corpus fetch-corpus gen-corpus \
	fuzz-compile fuzz-run fuzz-typeck fuzz-typeck_typed fuzz-number fuzz-structured fuzz-typeck_defs fuzz-determinism fuzz-roundtrip fuzz-splice fuzz-optdiff fuzz-metamorphic fuzz-spans fuzz-api fuzz-gcstress fuzz-host fuzz-serde_roundtrip \
	fuzz-triage-compile fuzz-triage-run fuzz-triage-typeck fuzz-triage-typeck_typed fuzz-triage-number fuzz-triage-structured fuzz-triage-typeck_defs fuzz-triage-determinism fuzz-triage-roundtrip fuzz-triage-splice fuzz-triage-optdiff fuzz-triage-metamorphic fuzz-triage-spans fuzz-triage-api fuzz-triage-gcstress fuzz-triage-host fuzz-triage-serde_roundtrip \
	fuzz-standalone \
	fuzz-standalone-compile fuzz-standalone-run fuzz-standalone-typeck fuzz-standalone-typeck_typed fuzz-standalone-number fuzz-standalone-structured fuzz-standalone-typeck_defs fuzz-standalone-determinism fuzz-standalone-roundtrip fuzz-standalone-splice fuzz-standalone-optdiff fuzz-standalone-metamorphic fuzz-standalone-spans fuzz-standalone-api fuzz-standalone-gcstress fuzz-standalone-host fuzz-standalone-serde_roundtrip

# ---------------------------------------------------------------------------
# AFL mode (real fuzzing; needs the AFL toolchain + nightly-free cargo-afl).
# ---------------------------------------------------------------------------

# Install the AFL runner (one-time).
setup-afl:
	cargo install cargo-afl

# Seed the corpus with real Luau programs (vendored conformance + upstream
# luau-lang/luau, MIT — see fuzz/CORPUS_ATTRIBUTION.md). run_afl.sh prefers this
# over the small curated seeds. Re-run any time to refresh.
# Build the full seed corpus: real scripts (fetch) + distilled generator seeds.
corpus: fetch-corpus gen-corpus

fetch-corpus:
	cd fuzz && ./scripts/fetch_corpus.sh

# Distill a seed corpus FROM the in-tree generators: valid generated programs as
# source seeds for compile/run/typeck, plus byte-inputs that decode to running /
# type-checking programs for optdiff/metamorphic/typeck_typed. Complements
# fetch-corpus (real scripts). run_afl.sh prefers corpus/<target>/.
gen-corpus:
	cd fuzz && cargo run --release --no-default-features --bin gen_corpus

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
fuzz-typeck_typed:
	cd fuzz && TARGET=typeck_typed FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-number:
	cd fuzz && TARGET=number FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-structured:
	cd fuzz && TARGET=structured FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-typeck_defs:
	cd fuzz && TARGET=typeck_defs FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-determinism:
	cd fuzz && TARGET=determinism FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-roundtrip:
	cd fuzz && TARGET=roundtrip FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-splice:
	cd fuzz && TARGET=splice FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-optdiff:
	cd fuzz && TARGET=optdiff FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-metamorphic:
	cd fuzz && TARGET=metamorphic FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-spans:
	cd fuzz && TARGET=spans FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-api:
	cd fuzz && TARGET=api FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-gcstress:
	cd fuzz && TARGET=gcstress FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-host:
	cd fuzz && TARGET=host FUZZ_SECS=$(SECS) ./scripts/run_afl.sh
fuzz-serde_roundtrip:
	cd fuzz && TARGET=serde_roundtrip FUZZ_SECS=$(SECS) ./scripts/run_afl.sh

# The full list of fuzz targets (shared by fuzz-all / fuzz-loop), ORDERED BY
# bugs found so far: run (os.time + adjustasize overflow + a verified upstream
# UBSan int-overflow), typeck_typed (getMutable type-alias), typeck
# (PromoteTypeLevels getMutable), determinism (the HashSet non-determinism) lead;
# the rest follow. The runtime/host/GC/serde targets (api gcstress host
# serde_roundtrip) are new — appended after the proven finders. fuzz-all does a
# single pass, so the proven finders go first.
FUZZ_TARGETS := run typeck_typed typeck determinism compile structured splice optdiff metamorphic roundtrip spans number typeck_defs api gcstress host serde_roundtrip

# Cycle every target through AFL once, giving each a time slice (default 300s — a
# bare `make fuzz-all` would otherwise sit on the first target forever).
# Override: `make fuzz-all SECS=600`.
fuzz-all:
	@secs="$(SECS)"; [ -n "$$secs" ] || secs=300; \
	for t in $(FUZZ_TARGETS); do \
	  echo ">>> AFL $$t for $${secs}s"; \
	  ( cd fuzz && AFL_AUTORESUME=1 TARGET=$$t FUZZ_SECS=$$secs ./scripts/run_afl.sh ) || true; \
	done

# Cycle every target INDEFINITELY, FUZZ_SECS each (default 300), looping forever
# until Ctrl+C. Each visit resumes that target's own corpus (AFL_AUTORESUME), so
# it's continuous coverage-guided fuzzing that fans time across all targets.
# The order is RESHUFFLED every round (perl shuffle, auto-seeded per process), so
# over many rounds every target gets equal expected time and no target is starved
# if you Ctrl+C mid-round — unlike fuzz-all's fixed bug-priority pass.
# Override the per-target slice: `make fuzz-loop SECS=600`.
fuzz-loop:
	@secs="$(SECS)"; [ -n "$$secs" ] || secs=300; \
	round=0; \
	trap 'echo "fuzz-loop stopped after $$round round(s)"; exit 0' INT; \
	while true; do \
	  round=$$((round+1)); \
	  echo "=========== fuzz-loop round $$round ($${secs}s/target, shuffled) ==========="; \
	  for t in $$(perl -MList::Util=shuffle -e 'print join(" ", shuffle @ARGV)' $(FUZZ_TARGETS)); do \
	    echo ">>> AFL $$t for $${secs}s"; \
	    ( cd fuzz && AFL_AUTORESUME=1 TARGET=$$t FUZZ_SECS=$$secs ./scripts/run_afl.sh ) || true; \
	  done; \
	done

# Minimize + bucket AFL crashes for a target.
fuzz-triage-compile:
	cd fuzz && TARGET=compile ./scripts/triage_afl_crashes.sh
fuzz-triage-run:
	cd fuzz && TARGET=run ./scripts/triage_afl_crashes.sh
fuzz-triage-typeck:
	cd fuzz && TARGET=typeck ./scripts/triage_afl_crashes.sh
fuzz-triage-typeck_typed:
	cd fuzz && TARGET=typeck_typed ./scripts/triage_afl_crashes.sh
fuzz-triage-number:
	cd fuzz && TARGET=number ./scripts/triage_afl_crashes.sh
fuzz-triage-structured:
	cd fuzz && TARGET=structured ./scripts/triage_afl_crashes.sh
fuzz-triage-typeck_defs:
	cd fuzz && TARGET=typeck_defs ./scripts/triage_afl_crashes.sh
fuzz-triage-determinism:
	cd fuzz && TARGET=determinism ./scripts/triage_afl_crashes.sh
fuzz-triage-roundtrip:
	cd fuzz && TARGET=roundtrip ./scripts/triage_afl_crashes.sh
fuzz-triage-splice:
	cd fuzz && TARGET=splice ./scripts/triage_afl_crashes.sh
fuzz-triage-optdiff:
	cd fuzz && TARGET=optdiff ./scripts/triage_afl_crashes.sh
fuzz-triage-metamorphic:
	cd fuzz && TARGET=metamorphic ./scripts/triage_afl_crashes.sh
fuzz-triage-spans:
	cd fuzz && TARGET=spans ./scripts/triage_afl_crashes.sh
fuzz-triage-api:
	cd fuzz && TARGET=api ./scripts/triage_afl_crashes.sh
fuzz-triage-gcstress:
	cd fuzz && TARGET=gcstress ./scripts/triage_afl_crashes.sh
fuzz-triage-host:
	cd fuzz && TARGET=host ./scripts/triage_afl_crashes.sh
fuzz-triage-serde_roundtrip:
	cd fuzz && TARGET=serde_roundtrip ./scripts/triage_afl_crashes.sh

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
fuzz-standalone-typeck_typed:
	cd fuzz && cargo build --no-default-features --bin typeck_typed
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/typeck_typed
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
fuzz-standalone-roundtrip:
	cd fuzz && cargo build --no-default-features --bin roundtrip
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/roundtrip
fuzz-standalone-splice:
	cd fuzz && cargo build --no-default-features --bin splice
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/splice
fuzz-standalone-optdiff:
	cd fuzz && cargo build --no-default-features --bin optdiff
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/optdiff
fuzz-standalone-metamorphic:
	cd fuzz && cargo build --no-default-features --bin metamorphic
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/metamorphic
fuzz-standalone-spans:
	cd fuzz && cargo build --no-default-features --bin spans
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/spans
fuzz-standalone-api:
	cd fuzz && cargo build --no-default-features --bin api
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/api
fuzz-standalone-gcstress:
	cd fuzz && cargo build --no-default-features --bin gcstress
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/gcstress
fuzz-standalone-host:
	cd fuzz && cargo build --no-default-features --bin host
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/host
fuzz-standalone-serde_roundtrip:
	cd fuzz && cargo build --no-default-features --bin serde_roundtrip
	cd fuzz && LUAUR_FUZZ_ITERS=$(ITERS) LUAUR_FUZZ_SEED=$(SEED) ./target/debug/serde_roundtrip

# Run every target's standalone smoke fuzz.
fuzz-standalone: fuzz-standalone-compile fuzz-standalone-run fuzz-standalone-typeck fuzz-standalone-typeck_typed fuzz-standalone-number fuzz-standalone-structured fuzz-standalone-typeck_defs fuzz-standalone-determinism fuzz-standalone-roundtrip fuzz-standalone-splice fuzz-standalone-optdiff fuzz-standalone-metamorphic fuzz-standalone-spans fuzz-standalone-api fuzz-standalone-gcstress fuzz-standalone-host fuzz-standalone-serde_roundtrip

# ---------------------------------------------------------------------------
# wasm fuzzing — replay the (native-AFL-evolved) corpus through the wasm32-wasip1
# build under wasmtime, to surface 32-bit/pointer-width bugs native can't (found
# the strtod stub null-deref). AFL can't instrument wasm, so coverage guidance
# comes from the native loop; wasm just executes. wasm forces panic=abort, so
# scripts/wasm_fuzz.sh CLASSIFIES aborts: luaD_throw/CompileError = normal Lua/
# compile errors (expected), anything else (asserts, null derefs) = a real bug.
# Needs: wasmtime + `rustup target add wasm32-wasip1`.
# ---------------------------------------------------------------------------
WASM_TARGETS := run typeck compile structured
WASM_LIMIT ?= 300
build-wasm:
	cd fuzz && for t in $(WASM_TARGETS); do \
	  cargo build --release --no-default-features --bin $$t --target wasm32-wasip1; done
fuzz-wasm: build-wasm
	cd fuzz && for t in $(WASM_TARGETS); do \
	  TARGET=$$t INPUTS=corpus/$$t LIMIT=$(WASM_LIMIT) ./scripts/wasm_fuzz.sh || true; done
