# luaur fuzzing

Coverage-guided fuzzing of luaur with [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz)
(libFuzzer). A Rust port of [Luau's `fuzz/` suite](https://github.com/luau-lang/luau/tree/master/fuzz):
one target per pipeline stage, plus a structured generator (the `proto.cpp`
analog) that emits valid Luau so inputs reach deep into the compiler and VM.

This crate is **detached from the main workspace** (its own `[workspace]`) and is
`publish = false`, so the nightly + libFuzzer toolchain it needs never touches a
normal `cargo build` or CI. For a no-nightly, CI-runnable generative fuzzer see
`crates/luaur-rt/tests/fuzz_generated.rs`.

## Targets

| target       | Luau analog          | what it fuzzes                                   |
|--------------|----------------------|--------------------------------------------------|
| `compile`    | `compiler.cpp`/`parser.cpp` | bytes → source → parse + compile          |
| `run`        | `kFuzzVM`            | compile + interrupt-bounded VM execution         |
| `typeck`     | `typeck.cpp`        | the static type checker (`luaur_rt::check`)      |
| `number`     | `number.cpp`        | numeric-literal parsers (`parse_double`/`_integer`) |
| `structured` | `proto.cpp`         | grammar-generated **valid** Luau → compile + run + check |

## The oracle

Every target asserts only that the implementation **never panics, aborts, hangs,
or exhibits UB** on any input — it must always return `Ok` or a structured `Err`.
Execution is bounded by an interrupt step-limit so a generated infinite loop
cannot hang the fuzzer. (A stronger oracle — differential testing of *output*
against reference Lua — is a natural next step.)

## Running

```sh
rustup toolchain install nightly          # one-time
cargo install cargo-fuzz                  # one-time

cargo +nightly fuzz run compile           # fuzz the compiler
cargo +nightly fuzz run structured        # fuzz with generated valid programs
cargo +nightly fuzz run run -- -max_total_time=300   # 5-minute soak of the VM

cargo +nightly fuzz list                  # all targets
```

A crash writes the reproducing input to `fuzz/artifacts/<target>/`; replay it
with `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>`.
