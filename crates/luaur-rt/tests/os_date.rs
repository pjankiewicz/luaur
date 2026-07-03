//! End-to-end `os.date` coverage through the runtime.
//!
//! The `%` specifiers used to be rendered through libc `strftime`; on
//! `wasm32-unknown-unknown` no libc exists, the import was an unresolved stub,
//! and any script formatting a date trapped with `unreachable` in the browser
//! (the same defect class as the former `string.format`/`snprintf`
//! dependency). Rendering is now pure Rust with C-locale strings, so these
//! assertions hold byte-identically on every target — including `%c`/`%x`/`%X`,
//! which MSVC's strftime used to render differently. `%z`/`%Z` are pinned in
//! the `strftime_directive` unit tests instead: their data legitimately
//! differs per target (the MSVC `tm` carries none).

use luaur_rt::{Lua, Result};

fn eval(src: &str) -> Result<String> {
    Lua::new().load(src).eval::<String>()
}

fn check(expr: &str, expected: &str) {
    assert_eq!(eval(&format!("return {expr}")).unwrap(), expected, "{expr}");
}

#[test]
fn utc_epoch() {
    check(r#"os.date("!%Y-%m-%d %H:%M:%S", 0)"#, "1970-01-01 00:00:00");
    check(r#"os.date("!%a %A %b %B", 0)"#, "Thu Thursday Jan January");
    check(r#"os.date("!%c", 0)"#, "Thu Jan  1 00:00:00 1970");
    check(r#"os.date("!%x %X", 0)"#, "01/01/70 00:00:00");
    check(r#"os.date("!%y %j %w %U %W", 0)"#, "70 001 4 00 00");
    check(r#"os.date("!%I %p", 0)"#, "12 AM");
    check(r#"os.date("!100%%", 0)"#, "100%");
}

#[test]
fn leap_day() {
    // os.time interprets the table via the pure-Rust timegm, so the
    // timestamp itself is deterministic on every target.
    check(
        r#"os.date("!%c", os.time({year=2024, month=2, day=29, hour=13, min=45, sec=56}))"#,
        "Thu Feb 29 13:45:56 2024",
    );
    check(
        r#"os.date("!%j %m %d %I %p", os.time({year=2024, month=2, day=29, hour=13, min=45, sec=56}))"#,
        "060 02 29 01 PM",
    );
    check(
        r#"os.date("!%j", os.time({year=2020, month=12, day=31, hour=12}))"#,
        "366",
    );
}

#[test]
fn week_numbers() {
    // 2023-01-01 was a Sunday (opens %U week 1), 2024-01-01 a Monday
    // (opens %W week 1).
    check(
        r#"os.date("!%w %U %W", os.time({year=2023, month=1, day=1, hour=12}))"#,
        "0 01 00",
    );
    check(
        r#"os.date("!%w %U %W", os.time({year=2024, month=1, day=1, hour=12}))"#,
        "1 00 01",
    );
}

#[test]
fn broken_down_table_round_trip() {
    let lua = Lua::new();
    lua.load(
        r#"
        local t = os.time({year=2024, month=2, day=29, hour=13, min=45, sec=56})
        local D = os.date("!*t", t)
        assert(D.year == 2024 and D.month == 2 and D.day == 29)
        assert(D.hour == 13 and D.min == 45 and D.sec == 56)
        assert(D.wday == 5 and D.yday == 60 and D.isdst == false)
        assert(os.time(D) == t)
        "#,
    )
    .exec()
    .unwrap();
}

#[test]
fn validation_and_edges() {
    let lua = Lua::new();
    // Unknown specifiers are rejected before rendering, exactly as upstream.
    for bad in [r#"os.date("%L")"#, r#"os.date("%9")"#, r#"os.date("%Ea")"#] {
        let err = lua
            .load(&format!("return {bad}"))
            .eval::<String>()
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid conversion specifier"),
            "{bad}: {err}"
        );
    }
    // A lone trailing '%' is literal; empty formats stay empty.
    check(r#"os.date("!%", 0)"#, "%");
    check(r#"os.date("!", 0)"#, "");
    // Pre-epoch local time is nil (upstream guard), but UTC accepts it.
    lua.load(r#"assert(os.date("", -1) == nil)"#)
        .exec()
        .unwrap();
    check(r#"os.date("!%Y %X", -1)"#, "1969 23:59:59");
}
