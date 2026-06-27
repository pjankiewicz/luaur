//! Pure-Rust `strtod` for wasm.
//!
//! `wasm32-unknown-unknown` (the web playground) ships no libc, so the VM's
//! `extern "C" { fn strtod }` (used by `luaO_str2d` for runtime string→number,
//! e.g. `tonumber("1.5")`) has no provider there. The build used to stub it to
//! `{ 0.0 }` *and never write `endptr`*, so `luaO_str2d` dereferenced a null
//! `endptr` — every wasm build crashed (debug) / returned `0.0` (release) on any
//! runtime string→number conversion. (Number *literals* are handled by the lexer,
//! so this only bit `tonumber`/coercion at runtime; native used real libc and was
//! never affected. Found by fuzzing the wasm32 target.)
//!
//! The scanning core ([`parse_c_double`]) is target-independent and unit-tested
//! natively; only the `#[no_mangle]` C entry point is wasm-gated. It provides the
//! symbol on `unknown` and, since (like the existing `strtoul` shim) it does not
//! clash with wasi-libc, on `wasm32-wasip1` too.

/// Scan a leading C `double` from `b` (the bytes of a NUL-terminated string, NUL
/// excluded). Returns `(value, consumed)` where `consumed` counts leading
/// whitespace plus the number, matching where C `strtod` leaves `endptr`.
/// `consumed == 0` means "no conversion" (C `strtod` sets `endptr == nptr`).
///
/// Decimal mantissa + optional exponent only: a `0x…` prefix consumes the leading
/// `0` and stops at `x`, so `luaO_str2d`'s hex path (`strtoul`) takes over — this
/// matches Lua built without a hex-float libc.
#[cfg(any(target_arch = "wasm32", test))]
pub fn parse_c_double(b: &[u8]) -> (f64, usize) {
    let mut i = 0usize;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let num_start = i;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let mut saw_digit = false;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        saw_digit = true;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            saw_digit = true;
        }
    }
    if !saw_digit {
        return (0.0, 0); // no conversion → endptr == nptr
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        if j < b.len() && b[j].is_ascii_digit() {
            i = j;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
        }
    }
    let val = core::str::from_utf8(&b[num_start..i])
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    (val, i)
}

/// `strtod(nptr, endptr)` for wasm (no libc on `unknown`). Parses a leading C
/// `double` from `nptr` and writes the first unconsumed byte to `*endptr`.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub unsafe extern "C" fn strtod(
    nptr: *const core::ffi::c_char,
    endptr: *mut *mut core::ffi::c_char,
) -> f64 {
    if nptr.is_null() {
        if !endptr.is_null() {
            *endptr = nptr as *mut core::ffi::c_char;
        }
        return 0.0;
    }
    let mut len = 0usize;
    while *nptr.add(len) != 0 {
        len += 1;
    }
    let bytes = core::slice::from_raw_parts(nptr as *const u8, len);
    let (val, consumed) = parse_c_double(bytes);
    if !endptr.is_null() {
        // consumed == 0 ⇒ no conversion ⇒ endptr == nptr (C semantics).
        *endptr = (nptr as *mut core::ffi::c_char).add(consumed);
    }
    val
}

#[cfg(test)]
mod tests {
    use super::parse_c_double;

    fn parse(s: &str) -> (f64, usize) {
        parse_c_double(s.as_bytes())
    }

    #[test]
    fn decimal_forms() {
        assert_eq!(parse("1.5"), (1.5, 3));
        assert_eq!(parse("0"), (0.0, 1));
        assert_eq!(parse("42"), (42.0, 2));
        assert_eq!(parse(".5"), (0.5, 2));
        assert_eq!(parse("12."), (12.0, 3));
        assert_eq!(parse("-3.14"), (-3.14, 5));
        assert_eq!(parse("+7"), (7.0, 2));
    }

    #[test]
    fn exponents() {
        assert_eq!(parse("1e10"), (1e10, 4));
        assert_eq!(parse("2.5E-3"), (2.5e-3, 6));
        // trailing 'e' with no exponent digits is not consumed
        assert_eq!(parse("1.5e"), (1.5, 3));
        assert_eq!(parse("1e+"), (1.0, 1));
    }

    #[test]
    fn whitespace_and_trailing() {
        assert_eq!(parse("  42"), (42.0, 4)); // leading ws counted in consumed
        assert_eq!(parse("3.14abc"), (3.14, 4)); // stops before 'abc'
    }

    #[test]
    fn hex_prefix_stops_at_x() {
        // luaO_str2d wants strtod to consume the leading "0" and stop at 'x',
        // so its strtoul hex path can take over.
        assert_eq!(parse("0x1f"), (0.0, 1));
    }

    #[test]
    fn no_conversion() {
        // consumed == 0 ⇒ caller leaves endptr == nptr and reports failure
        assert_eq!(parse(""), (0.0, 0));
        assert_eq!(parse("abc"), (0.0, 0));
        assert_eq!(parse("   "), (0.0, 0));
        assert_eq!(parse("+"), (0.0, 0));
        assert_eq!(parse(".e5"), (0.0, 0));
    }
}
