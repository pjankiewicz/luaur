# luaur roadmap

luaur is a Rust implementation of Roblox's [Luau](https://luau.org) — the
compiler, the register VM, and the static type checker. It is built in two
deliberate phases. Naming the phases is the point: it sets honest expectations
about what the code *is* today and where it is going.

## Phase 1 — Faithful 1:1 translation (current)

**Goal:** reproduce upstream Luau exactly — the same algorithms, data structures,
and control flow, translated function-for-function and file-for-file from the C++
source. Correctness is verified *against the reference*: the same bytecode format,
the ported conformance suite, and identical observable behavior.

**Why first.** A 1:1 port is **checkable**. Every function carries a pointer to
the C++ source it mirrors, so its behavior can be diffed against upstream rather
than trusted. That is what makes "faithful and accurate" a *verifiable* claim
instead of a hope — and it is the only honest starting point for re-implementing
a language other people's code depends on.

**The cost, stated plainly.** Faithfulness means mirroring C++'s *memory model*,
not just its logic: `TypeId = *const Type` into hand-rolled arenas, self-
referential structs wired together with raw pointers, and `unsafe` throughout the
hot paths (VM dispatch, the type checker). This reproduces C++'s performance
characteristics **and** its by-pointer hazards — and Rust's move semantics add a
few of their own (a self-referential struct that is address-stable in C++ can
dangle when it is *moved* in Rust). So in Phase 1, memory safety is a **contract**
upheld by discipline and testing, not yet a guarantee enforced by the compiler.

**How we hold the line in the meantime: out-test the original.** When an
implementation is built on raw memory, the answer is aggressive testing — the
strategy SQLite uses for C. luaur runs the translated unit + conformance suite on
every change, plus:

- a **coverage-guided fuzzing suite** (`fuzz/`, AFL / cargo-afl, optional ASan)
  porting Luau's own `fuzz/` targets — one per stage (compile, run, type-check,
  number parsing) plus a structured generator and a repeated-`check_with_definitions`
  target — with a toolchain-free standalone mode for plain `cargo` / CI;
- an in-tree, no-nightly **generative fuzzer** (`crates/luaur-rt/tests/fuzz_generated.rs`)
  that exercises compile + interrupt-bounded execution across Linux/macOS/Windows in CI;
- a **cache-free cold build** gate so latent compile errors cannot hide behind a warm cache.

The oracle is "must never panic, abort, hang, or exhibit UB — only `Ok` or a
structured `Err`." See `fuzz/README.md`.

## Phase 2 — Idiomatic, safe Rust (next)

**Goal:** incrementally replace the raw-pointer model with idiomatic, safe Rust,
so whole classes of memory bugs become **impossible by construction** rather than
merely tested into submission.

**Concretely, in order of value:**

1. Remove the **self-referential raw pointers** — the `wire_self_pointers`
   pattern and `*mut BuiltinTypes` / `*mut Frontend` back-pointers — in favor of
   safe ownership (owned sub-structs, `Rc`/`Arc`, or restructured borrows). This
   is where the current hazards bite (see *Known gaps*).
2. Migrate the hottest, most bug-prone subsystem — the type checker's
   `TypeId = *const Type` arena — toward **safe handles** (arena indices /
   slotmap) where the safety win justifies the divergence from C++ layout.
3. Drive `unsafe` down toward its irreducible core (the VM's `lua_State` FFI
   surface), shrinking the trusted base to the smallest auditable footprint.

**Sequencing.** Safety migrations land subsystem by subsystem, each one guarded
by the Phase 1 test + fuzz suite as the regression net — so the idiomatic version
**provably matches** the faithful one before it replaces it.

**The payoff:** the same Luau, but a Rust port that *cannot* segfault. That is the
strongest version of the "reliable, memory-safe" story — and the reason the port
exists in Rust at all.

## Status / known gaps

- **Phase 1 is substantially complete:** compiler, register VM, type checker, and
  standard library; passes the ported conformance suite; runs at roughly 0.8× the
  reference C++ Luau interpreter (JIT-free), and compiles bytecode ~1.3× faster.
- **Phase-2 motivators being tracked and fenced by tests** — e.g. an
  arena-lifetime use-after-free in `check_with_definitions` under repeated calls /
  larger definition files (issue #6): a `TypeId` whose backing arena is freed
  while the type checker still reads it. It is precisely the raw-pointer-lifetime
  class that Phase 2 eliminates by construction; until then it is reproduced,
  characterized, and guarded by the `typeck_defs` fuzz target.
