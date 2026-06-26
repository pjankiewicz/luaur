# luaur fuzzing (AFL)

Fuzzing of luaur with [AFL](https://github.com/rust-fuzz/afl.rs) (`cargo-afl`). A
Rust port of [Luau's `fuzz/` suite](https://github.com/luau-lang/luau/tree/master/fuzz):
one target per pipeline stage, plus a structured generator (the `proto.cpp`
analog) that emits valid Luau so inputs reach deep into the compiler and VM.

This crate is **detached from the main workspace** (its own `[workspace]`) and is
`publish = false`. Every target builds in **two modes**:

* **AFL mode** (default feature `afl-runtime`) — `cargo afl build` / `cargo afl
  fuzz` links the AFL fork-server runtime and gives you the real-time TUI.
* **Standalone mode** (`--no-default-features`) — a plain binary that needs **no
  AFL, nightly, or sanitizer toolchain**. It replays corpus/crash files, reads
  stdin, or generates pseudo-random inputs, catching panics. This is what runs in
  CI and for quick local smoke checks.

## Targets

| target        | Luau analog                 | what it fuzzes                                          |
|---------------|-----------------------------|--------------------------------------------------------|
| `compile`     | `compiler.cpp`/`parser.cpp` | bytes → source → parse + compile                       |
| `run`         | `kFuzzVM`                   | compile + interrupt-bounded VM execution               |
| `typeck`      | `typeck.cpp`                | the static type checker (`luaur_rt::check`)            |
| `number`      | `number.cpp`                | numeric-literal parsers (`parse_double`/`_integer`)   |
| `structured`  | `proto.cpp`                 | grammar-generated **valid** Luau → compile + run + check |
| `typeck_defs` | —                           | `check_with_definitions` hammered in-process (issue #6 shape) |
| `determinism` | —                           | metamorphic oracle: same source → identical result    |

## The oracle

Every target asserts only that the implementation **never panics, aborts, hangs,
or exhibits UB** on any input — it must always return `Ok` or a structured `Err`.
VM execution is bounded by an interrupt step-limit so a generated infinite loop
cannot hang the fuzzer. `determinism` additionally asserts that the same input
always produces the same result.

## AFL mode (the real-time TUI)

```sh
make setup-afl                    # one-time: cargo install cargo-afl

make fuzz-typeck                  # fuzz the type checker (AFL TUI)
make fuzz-compile                 # fuzz the compiler
make fuzz-run                     # soak the VM
# ...also: fuzz-number fuzz-structured fuzz-typeck_defs fuzz-determinism
```

`make fuzz-<target>` builds the AFL-instrumented binary, seeds an empty corpus,
and launches `cargo afl fuzz` with its live TUI. Extra AFL flags can be passed
through the script, e.g.:

```sh
cd fuzz && TARGET=run ./scripts/run_afl.sh -V 300    # 5-minute timed run
```

### ASan build (memory / leak detection)

Set `LUAUR_FUZZ_ASAN=1` to instrument with AddressSanitizer — it catches the
use-after-free / leak classes this project cares about (e.g. the repeated-call
SIGSEGV that motivated `typeck_defs`):

```sh
LUAUR_FUZZ_ASAN=1 make fuzz-typeck_defs
```

### Corpus & crashes

* Corpus accumulates in `fuzz/corpus/<target>/` (grows across runs; coverage
  compounds).
* Crash reproducers land in `fuzz/artifacts/afl/<target>/default/crashes/`.

Replay a crash in either mode:

```sh
cd fuzz
cargo afl build --bin typeck                                 # AFL binary
./target/debug/typeck artifacts/afl/typeck/default/crashes/id:000000,...

# or with the toolchain-free standalone binary:
cargo build --no-default-features --bin typeck
./target/debug/typeck <crash-file>
```

## Standalone mode (no toolchain — CI & quick checks)

```sh
make fuzz-standalone                       # all targets, deterministic
make fuzz-standalone-typeck                 # one target
make fuzz-standalone ITERS=200000 SEED=1    # override iteration count / seed
```

Or directly via the thin wrapper:

```sh
scripts/fuzz.sh                 # default: typeck, ITERS iterations
scripts/fuzz.sh compile 50000   # target + iteration count
scripts/fuzz.sh all             # every target
```

A panic prints the offending input as a reproducible hex dump and exits
non-zero. Feed that hex back (as a file) to either binary to reproduce.

## Crash triage

After an AFL run, minimize and bucket the crashes:

```sh
make fuzz-triage-typeck
```

Outputs under `fuzz/artifacts/afl/<target>/triage/`:

* `details.tsv` — one row per still-crashing input
* `summary.tsv` — buckets sorted by frequency
* `summary.txt` — human-readable report

Useful overrides (run from `fuzz/`):

```sh
SKIP_TMIN=1 MAX_CRASHES=50 TARGET=typeck ./scripts/triage_afl_crashes.sh
```
