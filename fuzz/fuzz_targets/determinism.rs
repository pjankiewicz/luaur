// Metamorphic / determinism oracle: the same source must produce identical
// results every time. A divergence means hidden nondeterminism (e.g. iteration
// order leaking into diagnostics, or an uninitialized read) — a correctness bug
// even when nothing crashes. Complements the "must not crash" targets.

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    let src = luaur_fuzz::generate(data);

    // Type-checking is a pure function of the source: same in, same out.
    let a = format!("{:?}", luaur_rt::check(&src));
    let b = format!("{:?}", luaur_rt::check(&src));
    assert_eq!(a, b, "check() is non-deterministic for:\n{src}");

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
    fuzz!(|data: &[u8]| {
        exercise_input(data);
    });
    #[cfg(not(feature = "afl-runtime"))]
    standalone_main(exercise_input);
}
