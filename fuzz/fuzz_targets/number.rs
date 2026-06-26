// Port of Luau's `fuzz/number.cpp`: fuzz the numeric-literal parsers (the
// schubfach-adjacent paths are bug-prone). They must never panic — only return
// a `ConstantNumberParseResult`.

#[cfg(feature = "afl-runtime")]
use afl::fuzz;

#[cfg(not(feature = "afl-runtime"))]
include!("standalone.rs");

use luaur_ast::functions::parse_double::parse_double;
use luaur_ast::functions::parse_integer::parse_integer;
use luaur_ast::functions::parse_integer_64::parse_integer_64;

fn exercise_input(data: &[u8]) {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut d = 0.0f64;
        let _ = parse_double(&mut d, s);
        // `parse_integer` handles binary/hex literals only (it asserts base 2 or
        // 16 — base-10 integers go through `parse_integer_64` / `parse_double`).
        let mut f = 0.0f64;
        let _ = parse_integer(&mut f, s, 2);
        let _ = parse_integer(&mut f, s, 16);
        let mut i = 0i64;
        let _ = parse_integer_64(&mut i, s, 10);
        let _ = parse_integer_64(&mut i, s, 16);
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
