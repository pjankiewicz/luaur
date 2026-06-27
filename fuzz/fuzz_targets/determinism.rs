// Metamorphic / determinism oracle: the same source must produce identical
// results every time. A divergence means hidden nondeterminism (e.g. iteration
// order leaking into diagnostics, or an uninitialized read) — a correctness bug
// even when nothing crashes. Complements the "must not crash" targets.

use std::cell::RefCell;

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

thread_local! {
    // Two INDEPENDENT reusable checkers. Comparing A-vs-B (rather than calling the
    // one-shot `check` twice) keeps the metamorphic oracle strong — two separate
    // frontends, each with its own caches and its own per-call hash seeds, must
    // still agree on the same input — while amortizing the expensive builtin
    // registration (the same optimization as the `typeck` target), so this oracle
    // runs at full throughput instead of paying a fresh frontend build per check.
    // (Cross-input state-bleed is a different property, covered by luaur-rt's
    // `checker_does_not_bleed_state_across_reuse` test.) `Checker::check` does not
    // catch_unwind, so a checker panic surfaces to AFL as a crash — also desirable.
    static CHECKER_A: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
    static CHECKER_B: RefCell<luaur_rt::Checker> = RefCell::new(luaur_rt::Checker::new());
}

fn check_two(src: &str) -> (String, String) {
    let a = CHECKER_A.with(|c| format!("{:?}", c.borrow_mut().check(src)));
    let b = CHECKER_B.with(|c| format!("{:?}", c.borrow_mut().check(src)));
    (a, b)
}

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate(data);

    // Type-checking is a pure function of the source: same in, same out.
    let (a, b) = check_two(&src);
    assert_eq!(a, b, "check() is non-deterministic for:\n{src}");

    // Type-RICH programs exercise inference + unification, where any leak of
    // iteration order (e.g. a pointer-keyed hash set) into diagnostics surfaces
    // as a divergent *result* — the strongest signal that the checker isn't pure.
    // (This is exactly the class of the HashSet<TypeId> nondeterminism fixed in
    // the filterMap/merge/refineLValue union paths.)
    let typed = luaur_fuzz::generate_typed(data);
    let (ta, tb) = check_two(&typed);
    assert_eq!(
        ta, tb,
        "check() is non-deterministic for typed program:\n{typed}"
    );

    // Compilation likewise: success/failure must not vary run to run.
    let c1 = {
        let lua = luaur_rt::Lua::new();
        lua.load(&src).set_name("fuzz").into_function().is_ok()
    };
    let c2 = {
        let lua = luaur_rt::Lua::new();
        lua.load(&src).set_name("fuzz").into_function().is_ok()
    };
    assert_eq!(c1, c2, "compile result differs across runs for:\n{src}");
}

fn main() {
    #[cfg(feature = "afl-runtime")]
    {
        luaur_fuzz::install_afl_panic_hook();
        fuzz_nohook!(|data: &[u8]| {
            exercise_input(data);
        });
    }
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
