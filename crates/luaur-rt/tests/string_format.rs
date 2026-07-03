//! End-to-end `string.format` coverage through the runtime.
//!
//! The numeric specifiers (`%d`, `%x`, `%f`, …) used to be forwarded to the
//! platform's C `snprintf`; on `wasm32-unknown-unknown` no libc exists, the
//! import was an unresolved stub, and any script touching them trapped with
//! `unreachable` in the browser while native builds sailed through. The
//! directive formatter is now pure Rust, so the same assertions hold on every
//! target — these tests pin the observable C-printf behaviour in-engine.

use luaur_rt::{Lua, Result};

fn eval(src: &str) -> Result<String> {
    Lua::new().load(src).eval::<String>()
}

fn check(expr: &str, expected: &str) {
    assert_eq!(eval(&format!("return {expr}")).unwrap(), expected, "{expr}");
}

#[test]
fn decimal() {
    check(r#"string.format("%d", 42)"#, "42");
    check(r#"string.format("%d", -42)"#, "-42");
    check(r#"string.format("%i", 7)"#, "7");
    check(r#"string.format("%d", 3.0)"#, "3"); // integral float coerces
    check(r#"string.format("%5d", 42)"#, "   42");
    check(r#"string.format("%-5d", 42)"#, "42   ");
    check(r#"string.format("%05d", 42)"#, "00042");
    check(r#"string.format("%05d", -42)"#, "-0042");
    check(r#"string.format("%+d", 42)"#, "+42");
    check(r#"string.format("% d", 42)"#, " 42");
    check(r#"string.format("%.5d", 42)"#, "00042");
    check(r#"string.format("%8.5d", 42)"#, "   00042");
    check(r#"string.format("%.0d", 0)"#, "");
    check(r#"string.format("%%%d %010d", 10, 23)"#, "%10 0000000023");
    check(
        r#"string.format("%d", -9007199254740991)"#,
        "-9007199254740991",
    );
}

#[test]
fn unsigned_octal_hex() {
    check(r#"string.format("%u", 42)"#, "42");
    check(r#"string.format("%x", 255)"#, "ff");
    check(r#"string.format("%X", 255)"#, "FF");
    check(r#"string.format("%o", 8)"#, "10");
    check(r#"string.format("%#x", 255)"#, "0xff");
    check(r#"string.format("%#X", 255)"#, "0XFF");
    check(r#"string.format("%#o", 8)"#, "010");
    check(r#"string.format("%#010x", 255)"#, "0x000000ff");
    check(r#"string.format("%08x", 255)"#, "000000ff");
    // negative values wrap to the full 64-bit range (Luau semantics)
    check(
        r#"string.format("%o %u %x %X", -1, -1, -1, -1)"#,
        "1777777777777777777777 18446744073709551615 ffffffffffffffff FFFFFFFFFFFFFFFF",
    );
}

#[test]
fn fixed_floats() {
    check(r#"string.format("%f", 0)"#, "0.000000");
    check(r#"string.format("%f", 10.3)"#, "10.300000");
    check(r#"string.format("%.2f", 1.5)"#, "1.50");
    check(r#"string.format("%.0f", 2.5)"#, "2"); // round-half-even
    check(r#"string.format("%.0f", 3.5)"#, "4");
    check(r#"string.format("%#.0f", 5)"#, "5.");
    check(r#"string.format("%010.2f", -1.5)"#, "-000001.50");
    check(r#"string.format("%-8.2f|", 1.5)"#, "1.50    |");
    check(r#"string.format("%f", -0.0)"#, "-0.000000");
    // the longest number that can be formatted
    assert!(
        eval(r#"return string.format("%99.99f", -1e308)"#)
            .unwrap()
            .len()
            >= 100
    );
}

#[test]
fn scientific_floats() {
    check(r#"string.format("%e", 1.5)"#, "1.500000e+00");
    check(r#"string.format("%E", -1.5)"#, "-1.500000E+00");
    check(r#"string.format("%.0e", 12345)"#, "1e+04");
    check(r#"string.format("%e", 0)"#, "0.000000e+00");
    check(r#"string.format("%.2e", 1e308)"#, "1.00e+308");
    check(r#"string.format("%.2e", 0.000123)"#, "1.23e-04");
}

#[test]
fn general_floats() {
    check(r#"string.format("%g", 100000)"#, "100000");
    check(r#"string.format("%g", 1000000)"#, "1e+06");
    check(r#"string.format("%g", 0.0001)"#, "0.0001");
    check(r#"string.format("%g", 0.00001)"#, "1e-05");
    check(r#"string.format("%g", 0)"#, "0");
    check(r#"string.format("%g", 0.5)"#, "0.5");
    check(r#"string.format("%.3g", 1234.5)"#, "1.23e+03");
    check(r#"string.format("%#g", 1)"#, "1.00000");
    check(r#"string.format("%G", 1e-10)"#, "1E-10");
}

#[test]
fn nonfinite_floats() {
    check(r#"string.format("%f", 1/0)"#, "inf");
    check(r#"string.format("%f", -1/0)"#, "-inf");
    check(r#"string.format("%+f", 1/0)"#, "+inf");
    check(r#"string.format("%E", 1/0)"#, "INF");
    check(r#"string.format("%8f", 1/0)"#, "     inf");
    // NaN formats without a sign on every target (hardware NaN sign bits
    // differ between x86, ARM and wasm).
    check(r#"string.format("%f", 0/0)"#, "nan");
    check(r#"string.format("%G", 0/0)"#, "NAN");
}

#[test]
fn chars_and_strings() {
    check(r#"string.format("%c", 65)"#, "A");
    check(r#"string.format("%5c", 65)"#, "    A");
    check(r#"string.format("%-5c|", 65)"#, "A    |");
    check(r#"string.format("%5s", "ab")"#, "   ab");
    check(r#"string.format("%-5s|", "ab")"#, "ab   |");
    check(r#"string.format("%.2s", "abc")"#, "ab");
    check(r#"string.format("%5.2s", "abc")"#, "   ab");
    // %c of zero and embedded NULs survive
    assert_eq!(
        eval(r#"return string.format("%c%c%c%c", 1, 0, 2, 3)"#)
            .unwrap()
            .as_bytes(),
        b"\x01\x00\x02\x03"
    );
}

#[test]
fn quoted() {
    check(
        r#"string.format("%q", 'he said "hi"')"#,
        r#""he said \"hi\"""#,
    );
    check(r#"string.format("%q", "a\nb")"#, "\"a\\\nb\"");
    check(r#"string.format("%q", "a\rb")"#, r#""a\rb""#);
    check(r#"string.format("%q", "a\0b")"#, r#""a\000b""#);
    check(r#"string.format("%q", "back\\slash")"#, r#""back\\slash""#);
}

#[test]
fn invalid_formats_error() {
    let lua = Lua::new();
    for bad in [
        r#"string.format("%?", 1)"#,
        r#"string.format("%.123d", 1)"#,
        r#"string.format("%##################d", 1)"#,
        r#"string.format("%d")"#,
    ] {
        assert!(
            lua.load(&format!("return {bad}")).eval::<String>().is_err(),
            "{bad} should error"
        );
    }
}

/// The production repro: a game-cart-style tick function whose HUD text is
/// composed with numeric specifiers. This used to trap `unreachable` on
/// wasm32-unknown-unknown while passing every native check.
#[test]
fn cart_tick_repro() {
    let out = eval(
        r#"
        local score, health, t = 12345, 0.75, 61.5
        local function hud()
            return string.format("SCORE %08d  HP %3d%%  T %.1fs  0x%04X", score, health * 100, t, 48879)
        end
        return hud()
        "#,
    )
    .unwrap();
    assert_eq!(out, "SCORE 00012345  HP  75%  T 61.5s  0xBEEF");
}
