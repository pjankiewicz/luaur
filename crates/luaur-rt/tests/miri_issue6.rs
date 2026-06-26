//! Miri-only reproduction of issue #6: a SIGSEGV in `check_with_definitions`
//! that manifests on rustc 1.88 (and not on newer toolchains) — i.e. a genuine
//! layout-dependent UB that a newer compiler's enum/struct layout happens to
//! hide. Running this under Miri pinpoints the *first* UB on the exact execution
//! path (transmute, uninitialized read, out-of-bounds, or aliasing violation).
//!
//!   cargo +nightly miri test -p luaur-rt --features typecheck issue6 -- --nocapture
//!
//! This test is not a correctness oracle — it asserts nothing about the result.
//! It exists purely so Miri walks the crashing path and reports the UB.
#![cfg(feature = "typecheck")]

#[test]
fn issue6_check_with_definitions_does_not_trip_miri() {
    // The original crash: `return true` checked against a freshly-registered
    // builtin environment, which drove the return-pack unifier through the
    // `TypeChecker::tryUnify(TypePackId, TypePackId)` overload. That overload used
    // to reinterpret-cast the pack ids to type ids and run *type* unification on a
    // `TypePackVar` — layout-dependent UB that SIGSEGVed on rustc 1.88 and trips
    // Miri's "invalid enum tag" check. These calls must complete cleanly.
    let _ = luaur_rt::check_with_definitions("return true", "");
    let _ =
        luaur_rt::check_with_definitions("return true", "declare function log(message: any): ()");
    // Repeated calls reuse global registration paths; exercise that too.
    let _ = luaur_rt::check_with_definitions("local x = 1 return x", "");
}
