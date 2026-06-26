// Metamorphic / determinism oracle: the same source must produce identical
// results every time. A divergence means hidden nondeterminism (e.g. iteration
// order leaking into diagnostics, or an uninitialized read) — a correctness bug
// even when nothing crashes. Complements the "must not crash" targets.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
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
});
