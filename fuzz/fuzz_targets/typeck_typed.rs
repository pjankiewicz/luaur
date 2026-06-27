// Type-checker fuzzing with TYPE-RICH programs: type annotations, aliases,
// generics, unions/intersections/optionals, function & table types, type
// assertions, and partially-typed code. The untyped `typeck` target barely
// exercises the type system (luaur's unique surface) — this generator drives
// inference + the annotation/unification/cycle machinery where every serious bug
// found in this repo lived (#6, the DFG aliasing failures, the shared_seen leak).
//
// Oracle: never panic/abort/hang — only Ok or a structured diagnostic.

use std::cell::RefCell;

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

thread_local! {
    // Register the Luau builtins ONCE and reuse the global environment across
    // inputs (same amortization as the `typeck` target): `luaur_rt::check`
    // rebuilds the frontend + re-checks the whole @luau definition file every
    // call, which dominated throughput; the reusable Checker drops the per-input
    // cost to just parsing + checking the generated program.
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate_typed(data);
    CHECKER.with(|c| {
        let _ = c.borrow_mut().check(&src);
    });
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
