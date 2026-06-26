// Port of Luau's `fuzz/compiler.cpp` (+ parser.cpp): feed arbitrary bytes as
// source and compile. The compiler must never panic/crash — only return
// `Ok(function)` or `Err(SyntaxError)`. libFuzzer's coverage feedback explores
// the parser + bytecode compiler.
#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(src) = std::str::from_utf8(data) {
        let lua = luaur_rt::Lua::new();
        let _ = lua.load(src).set_name("fuzz").into_function();
    }
});
