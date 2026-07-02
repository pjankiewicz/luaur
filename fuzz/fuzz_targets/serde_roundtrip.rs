// Serde round-trip target: `to_value` (Rust -> Lua `Value`) followed by
// `from_value` (Lua `Value` -> Rust) must be the identity. luaur-rt's serde
// bridge is a surface embedders feed UNTRUSTED data through, and it's a pile of
// value-shape heuristics (integer-vs-number reconstruction, the `null` sentinel
// table, array-vs-map detection) — exactly where a round-trip silently corrupts
// or drops data.
//
// The value model is an externally-tagged enum `J`: every node serializes as
// `{VariantName = payload}`, so the array-vs-map / empty-table ambiguity that
// normally makes a Lua round-trip oracle unsound cannot arise — the tag tells
// the deserializer which shape to expect. Numbers are `i32` only (exact in the
// f64 Lua carries, and totally ordered — no NaN), keeping equality total.
//
// Oracle: a round-trip that SUCCEEDS must be the identity. A `from_value` error
// is treated as inconclusive (a legitimate limit, e.g. recursion depth), not a
// bug — the strong signal is "succeeded but the value changed". A panic anywhere
// is a crash finding (the standalone harness / AFL catch it).

use std::cell::RefCell;
use std::collections::BTreeMap;

use arbitrary::Arbitrary;
use serde::{Deserialize, Serialize};

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_rt::{Lua, LuaSerdeExt};

#[derive(Debug, Clone, PartialEq, Eq, Arbitrary, Serialize, Deserialize)]
enum J {
    Null,
    Bool(bool),
    Int(i32),
    Str(String),
    Arr(Vec<J>),
    Map(BTreeMap<String, J>),
}

thread_local! {
    // Reuse ONE Lua state across inputs (like the `typeck` target). A fresh
    // `Lua::new()` per input is not just slower — it accumulates one entry per
    // state in luaur-rt's serde sentinel thread-local (the `null`/array-metatable
    // cache is keyed by state pointer and never removed on state drop), and at
    // PROCESS EXIT that pile-up drops in an order that accesses an already-
    // destroyed TLS, aborting the process ("thread local panicked on drop"). That
    // is a real luaur-rt bug (a long-lived process creating many short-lived
    // Lua+serde states leaks and aborts at exit); a single reused state has one
    // sentinel entry, which tears down cleanly. `to_value`/`from_value` are
    // independent per call, so reuse doesn't weaken the round-trip oracle.
    static LUA: RefCell<Lua> = RefCell::new(Lua::new());
}

fn exercise_input(data: &[u8]) {
    let mut u = arbitrary::Unstructured::new(data);
    let Ok(j) = J::arbitrary(&mut u) else {
        return;
    };

    LUA.with(|cell| {
        let lua = cell.borrow();

        // Rust -> Lua Value.
        let Ok(value) = lua.to_value(&j) else {
            return; // couldn't build the Lua value — nothing to compare
        };

        // Lua Value -> Rust. A deserialize error is inconclusive (see the header).
        if let Ok(back) = lua.from_value::<J>(value) {
            assert_eq!(
                j, back,
                "serde round-trip changed the value:\n  in  = {j:?}\n  out = {back:?}"
            );
        }
    });
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    afl::fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
