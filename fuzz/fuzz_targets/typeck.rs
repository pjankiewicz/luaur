// Port of Luau's `fuzz/typeck.cpp`: run arbitrary source through the static type
// checker (luaur-analysis, via `luaur_rt::Checker`). The checker must never
// panic/crash — only return `Ok(())` or `Err(Vec<TypeDiagnostic>)`. (Several of
// the bugs hardened in this repo lived in the analysis layer, so this target is
// especially valuable.)

use std::cell::RefCell;

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

thread_local! {
    // Register the Luau builtins ONCE and reuse the global environment across
    // inputs. `luaur_rt::check` rebuilds the frontend + re-checks the whole @luau
    // definition file every call, which dominated throughput (~50 exec/s); the
    // reusable Checker drops the per-input cost to just parsing + checking the
    // input (orders of magnitude faster).
    static CHECKER: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn exercise_input(data: &[u8]) {
    if let Ok(src) = std::str::from_utf8(data) {
        CHECKER.with(|c| {
            let _ = c.borrow_mut().check(src);
        });
    }
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
