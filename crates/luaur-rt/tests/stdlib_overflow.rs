//! Regression tests for integer-overflow / stack-safety defects in the stdlib,
//! all found by the metadata-driven `api` fuzz target (it calls every builtin
//! with boundary-value arguments — INT_MIN/MAX, 2^53, NaN, huge counts — which
//! the from-scratch grammar generators never reach).
//!
//! Three are signed-overflow UB shared with upstream C++ Luau (verified against
//! the UBSan build): `buffer.copy` (lbuflib.cpp:257), `buffer.fill`
//! (lbuflib.cpp:278), `table.concat` (ltablib.cpp:232). Two are port-only: the
//! Rust port's checked arithmetic panicked where C++'s unsigned wraparound is
//! well-defined (`math.randomseed` -> `pcg32_seed`), and a translation slip in
//! `table.unpack` added one to the element count BEFORE the `>= INT_MAX` guard,
//! wrapping it to 0 and letting the push loop overrun the stack into an
//! `api_incr_top` assert (SIGTRAP).
//!
//! Under `cfg(test)` overflow checks are ARMED, so a reverted fix either aborts
//! the process (uncaught panic / SIGTRAP) or surfaces an "overflow" message — the
//! assertions below catch both. The `table.move` huge-range hang found alongside
//! these is intentionally NOT covered: C++ Luau loops the same way, so matching it
//! is correct behavior, not a bug.

use luaur_rt::Lua;

/// Run `body` inside a pcall and return the error message (empty on success). A
/// builtin that panics on overflow surfaces here as a Lua error string containing
/// "overflow"; a builtin that traps (LUAU_ASSERT) kills the process instead — so
/// either failure mode fails the test.
fn pcall_err(body: &str) -> String {
    let lua = Lua::new();
    let code = format!(
        "local ok, err = pcall(function() {body} end)\n\
         if ok then return '' else return tostring(err) end"
    );
    lua.load(&code)
        .set_name("regress")
        .eval::<String>()
        .expect("chunk should compile and run under pcall")
}

fn assert_no_overflow(what: &str, err: &str) {
    assert!(
        !err.contains("overflow") && !err.contains("attempt to"),
        "{what}: builtin panicked on a boundary argument instead of handling it: {err:?}"
    );
}

#[test]
fn math_randomseed_negative_does_not_overflow() {
    // seed = luaL_checkinteger(...) as u64 -> 0xFFFF... ; pcg32_seed `*state +=`
    // must wrap (as C++ does), not panic.
    let err = pcall_err("math.randomseed(-1)");
    assert_no_overflow("math.randomseed(-1)", &err);
    assert!(
        err.is_empty(),
        "math.randomseed(-1) should succeed: {err:?}"
    );
    // a couple more hostile seeds
    for s in ["-2147483648", "2147483647", "4294967295"] {
        let e = pcall_err(&format!("math.randomseed({s})"));
        assert_no_overflow("math.randomseed", &e);
    }
}

#[test]
fn buffer_copy_intmin_source_offset() {
    // arg-5 default `int(slen) - soffset` is evaluated eagerly; soffset = INT_MIN
    // makes it overflow upstream. Must reject with the domain error, not panic.
    let err = pcall_err("buffer.copy(buffer.create(8), 0, buffer.create(8), -2147483648)");
    assert_no_overflow("buffer.copy", &err);
    assert!(
        err.is_empty() || err.contains("out of bounds"),
        "buffer.copy unexpected error: {err:?}"
    );
}

#[test]
fn buffer_fill_intmin_offset() {
    let err = pcall_err("buffer.fill(buffer.create(8), -2147483648, 1)");
    assert_no_overflow("buffer.fill", &err);
    assert!(
        err.is_empty() || err.contains("out of bounds"),
        "buffer.fill unexpected error: {err:?}"
    );
}

#[test]
fn table_concat_intmin_start_index() {
    // addfield's `cast_to(unsigned, i - 1)` fast-path test overflows for i = INT_MIN.
    let err = pcall_err("return table.concat({1, 2, 3}, '', -2147483648)");
    assert_no_overflow("table.concat", &err);
    // It must take the slow path and reject the absent element cleanly.
    assert!(
        err.contains("invalid value") || err.is_empty(),
        "table.concat unexpected error: {err:?}"
    );
}

#[test]
fn table_unpack_full_int_range_rejected() {
    // i = INT_MIN, e = INT_MAX: element-count-minus-one is 0xFFFF_FFFF and must be
    // caught by the `>= INT_MAX` guard. A revert wraps it to 0, passes the guard,
    // and overruns the stack into a SIGTRAP (which would kill this process).
    let err = pcall_err("return table.unpack({1, 2, 3}, -2147483648, 2147483647)");
    assert!(
        err.contains("too many results"),
        "table.unpack must reject a full-range request: {err:?}"
    );
}

#[test]
fn table_find_intmax_element_does_not_overflow() {
    // tfind walks `i = init; i += 1` until it hits a nil slot. If the table has a
    // non-matching element at INT_MAX, the increment overflows (upstream UB
    // ltablib.cpp:533). Searching for a value the INT_MAX slot does NOT hold must
    // return nil cleanly, not panic.
    let err = pcall_err("return table.find({[2147483647] = false}, true, 2147483647)");
    assert_no_overflow("table.find", &err);
    assert!(
        err.is_empty(),
        "table.find should return nil cleanly: {err:?}"
    );
    // and the normal find still works
    let lua = Lua::new();
    let idx: i64 = lua
        .load("return table.find({10, 20, 30}, 20)")
        .eval()
        .expect("table.find ordinary path");
    assert_eq!(idx, 2);
}

#[test]
fn ordinary_stdlib_calls_still_work() {
    // The wrapping/guard changes must not disturb the normal paths.
    let lua = Lua::new();
    let sum: i64 = lua
        .load("local b = buffer.create(8); buffer.fill(b, 0, 7); return buffer.readu8(b, 3)")
        .eval()
        .expect("buffer.fill/readu8 ordinary path");
    assert_eq!(sum, 7);
    let s: String = lua
        .load("return table.concat({'a', 'b', 'c'}, '-')")
        .eval()
        .expect("table.concat ordinary path");
    assert_eq!(s, "a-b-c");
    let n: i64 = lua
        .load("return select('#', table.unpack({10, 20, 30}))")
        .eval()
        .expect("table.unpack ordinary path");
    assert_eq!(n, 3);
}
