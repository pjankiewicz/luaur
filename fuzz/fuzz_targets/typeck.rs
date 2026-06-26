// Port of Luau's `fuzz/typeck.cpp`: run arbitrary source through the static type
// checker (luaur-analysis, via `luaur_rt::check`). The checker must never
// panic/crash — only return `Ok(())` or `Err(Vec<TypeDiagnostic>)`. (Several of
// the bugs hardened in this repo lived in the analysis layer, so this target is
// especially valuable.)

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    if let Ok(src) = std::str::from_utf8(data) {
        let _ = luaur_rt::check(src);
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
