//! Regression test for an integer overflow in `os.time` (`os_timegm`).
//!
//! The Julian-day computation in `luaur-vm/src/functions/os_timegm.rs` did its
//! arithmetic in C-`int` (i32). An out-of-range date table — e.g.
//! `os.time{year = 2^31-1}` — overflowed intermediates like `365 * y` and
//! `julianday * 86400`. With overflow checks on (debug builds / the fuzz profile)
//! that aborted the process; in plain release it silently wrapped to a garbage
//! timestamp. Found by the fuzzer running a mutated copy of the `datetime`
//! conformance test. The fix widens the computation to i64, which holds every
//! value reachable from i32 inputs.

use luaur_rt::Lua;

#[test]
fn os_time_known_date_is_correct() {
    let lua = Lua::new();
    // 2000-01-01 00:00:00 UTC is exactly 946684800 seconds since the epoch — the
    // i64 widening must not change the result for ordinary dates.
    let t: i64 = lua
        .load("return os.time({year=2000, month=1, day=1, hour=0, min=0, sec=0})")
        .eval()
        .expect("os.time of a valid date should evaluate");
    assert_eq!(t, 946_684_800, "os.time(2000-01-01 UTC)");
}

#[test]
fn os_time_extreme_year_does_not_overflow() {
    let lua = Lua::new();
    // Under cfg(test) overflow checks are ARMED, so a regression (i32 arithmetic)
    // aborts the process here — simply completing is the assertion. The result
    // value is unspecified for such an absurd date; we only require no panic.
    let r: luaur_rt::Result<i64> = lua
        .load("return os.time({year=2147483647, month=1, day=1, hour=0, min=0, sec=0})")
        .eval();
    assert!(
        r.is_ok(),
        "os.time with an extreme year must not abort: {r:?}"
    );

    // The negative extreme (and a few other hostile fields) likewise.
    for src in [
        "return os.time({year=-2147483648, month=1, day=1, hour=0, min=0, sec=0})",
        "return os.time({year=2024, month=2147483647, day=1, hour=0, min=0, sec=0})",
        "return os.time({year=2024, month=1, day=2147483647, hour=2147483647, min=2147483647, sec=2147483647})",
    ] {
        // Must return cleanly (Ok or a Lua error) — never abort.
        let _: luaur_rt::Result<i64> = lua.load(src).eval();
    }
}
