// Port of Luau's `fuzz/number.cpp`: fuzz the numeric-literal parsers (the
// schubfach-adjacent paths are bug-prone). They must never panic — only return
// a `ConstantNumberParseResult`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use luaur_ast::functions::parse_double::parse_double;
use luaur_ast::functions::parse_integer::parse_integer;
use luaur_ast::functions::parse_integer_64::parse_integer_64;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let mut d = 0.0f64;
        let _ = parse_double(&mut d, s);
        let mut f = 0.0f64;
        let _ = parse_integer(&mut f, s, 10);
        let _ = parse_integer(&mut f, s, 16);
        let mut i = 0i64;
        let _ = parse_integer_64(&mut i, s, 10);
        let _ = parse_integer_64(&mut i, s, 16);
    }
});
