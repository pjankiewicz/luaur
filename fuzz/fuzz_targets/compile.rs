// Port of Luau's `fuzz/compiler.cpp` (+ parser.cpp): feed arbitrary bytes as
// source and compile. The compiler must never panic/crash — only return
// `Ok(function)` or `Err(SyntaxError)`. AFL's coverage feedback explores the
// parser + bytecode compiler.

#[cfg(feature = "afl-runtime")]
use afl::fuzz_nohook;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

fn exercise_input(data: &[u8]) {
    if let Ok(src) = std::str::from_utf8(data) {
        let lua = luaur_rt::Lua::new();
        let _ = lua.load(src).set_name("fuzz").into_function();
    }
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
