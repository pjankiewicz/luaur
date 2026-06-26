// Port of Luau's `fuzz/typeck.cpp`: run arbitrary source through the static type
// checker (luaur-analysis, via `luaur_rt::check`). The checker must never
// panic/crash — only return `Ok(())` or `Err(Vec<TypeDiagnostic>)`. (Several of
// the bugs hardened in this repo lived in the analysis layer, so this target is
// especially valuable.)
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(src) = std::str::from_utf8(data) {
        let _ = luaur_rt::check(src);
    }
});
