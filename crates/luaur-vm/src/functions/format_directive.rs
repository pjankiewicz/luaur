//! Pure-Rust formatting of a single scanned `string.format` directive.
//!
//! The faithful translation of `str_format` used to forward each numeric /
//! `%c` / padded-`%s` directive to the platform's C `snprintf`. Native builds
//! bind libc (and `wasm32-wasip1` binds wasi-libc), but `wasm32-unknown-unknown`
//! ships no libc at all, so the `snprintf` symbol became an unresolved `env`
//! import and every `string.format("%d", …)` trapped with `unreachable` in the
//! browser. These helpers reproduce the C `printf` behaviour Luau relies on —
//! flags `- + space # 0`, width, precision, and the `c d i u o x X e E f g G s`
//! conversions — in portable Rust, so the output is byte-identical on every
//! target and no libc is required.
//!
//! Behaviour notes (kept deliberately platform-independent):
//! - floats are formatted with correct rounding at every precision (Rust's
//!   exact `core::fmt` paths), matching C's round-half-even;
//! - NaN never takes a `-` sign (the hardware sign bit of a freshly produced
//!   NaN differs between x86, ARM and wasm; musl makes the same choice), while
//!   the `+`/space flags still apply;
//! - the C-undefined corners are resolved the way glibc/musl resolve them:
//!   `+`/space are ignored for the unsigned conversions, `0` padding is
//!   ignored for `%c`/`%s`, and `0` is ignored whenever `-` is present or an
//!   integer precision is given.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// A parsed `%[flags][width][.precision]` prefix of one format directive
/// (everything between the `%` and the conversion character), as validated
/// and bounded by `scanformat` (width and precision are at most two digits).
pub(crate) struct FormatSpec {
    /// `-` — left-justify within the field width.
    pub left: bool,
    /// `+` — force a sign on signed conversions.
    pub plus: bool,
    /// ` ` — space in place of a `+` sign on signed conversions.
    pub space: bool,
    /// `#` — alternate form (`0x` prefix, forced octal leading zero, forced
    /// decimal point, kept trailing zeros for `%g`).
    pub alt: bool,
    /// `0` — pad the field with zeros after the sign/prefix.
    pub zero: bool,
    /// Minimum field width (0 when absent).
    pub width: usize,
    /// Precision (`None` when no `.` was given; `Some(0)` for a bare `.`).
    pub precision: Option<usize>,
}

/// Parse the flag/width/precision bytes of a directive (the bytes between the
/// `%` and the conversion character, exactly as `scanformat` collected them).
pub(crate) fn parse_format_spec(spec: &[u8]) -> FormatSpec {
    let mut fs = FormatSpec {
        left: false,
        plus: false,
        space: false,
        alt: false,
        zero: false,
        width: 0,
        precision: None,
    };
    let mut i = 0;
    while i < spec.len() {
        match spec[i] {
            b'-' => fs.left = true,
            b'+' => fs.plus = true,
            b' ' => fs.space = true,
            b'#' => fs.alt = true,
            b'0' => fs.zero = true,
            _ => break,
        }
        i += 1;
    }
    while i < spec.len() && spec[i].is_ascii_digit() {
        fs.width = fs.width * 10 + (spec[i] - b'0') as usize;
        i += 1;
    }
    if i < spec.len() && spec[i] == b'.' {
        i += 1;
        let mut p = 0usize;
        while i < spec.len() && spec[i].is_ascii_digit() {
            p = p * 10 + (spec[i] - b'0') as usize;
            i += 1;
        }
        fs.precision = Some(p);
    }
    fs
}

/// Lay `prefix` (sign / `0x`) and `body` (digits) out in a field of
/// `spec.width`: left-justified with trailing spaces for `-`, zero-filled
/// between prefix and body for `0` (when `allow_zero_pad`), otherwise
/// space-filled on the left.
fn assemble(spec: &FormatSpec, prefix: &[u8], body: &[u8], allow_zero_pad: bool) -> Vec<u8> {
    let content = prefix.len() + body.len();
    let pad = spec.width.saturating_sub(content);
    let mut out = Vec::with_capacity(content + pad);
    if spec.left {
        out.extend_from_slice(prefix);
        out.extend_from_slice(body);
        out.resize(content + pad, b' ');
    } else if spec.zero && allow_zero_pad {
        out.extend_from_slice(prefix);
        out.resize(prefix.len() + pad, b'0');
        out.extend_from_slice(body);
    } else {
        out.resize(pad, b' ');
        out.extend_from_slice(prefix);
        out.extend_from_slice(body);
    }
    out
}

/// Zero-extend `digits` on the left to the integer precision, honouring the
/// C rule that a zero value with an explicit precision of zero prints nothing.
fn apply_int_precision(digits: &mut String, precision: Option<usize>, is_zero: bool) {
    if let Some(p) = precision {
        if is_zero && p == 0 {
            digits.clear();
        } else if digits.len() < p {
            let mut padded = String::with_capacity(p);
            for _ in 0..(p - digits.len()) {
                padded.push('0');
            }
            padded.push_str(digits);
            *digits = padded;
        }
    }
}

/// `%d` / `%i` of an `i64` (C `%lld` semantics).
pub(crate) fn format_int(spec: &FormatSpec, v: i64) -> Vec<u8> {
    let mut digits = format!("{}", v.unsigned_abs());
    apply_int_precision(&mut digits, spec.precision, v == 0);
    let prefix: &[u8] = if v < 0 {
        b"-"
    } else if spec.plus {
        b"+"
    } else if spec.space {
        b" "
    } else {
        b""
    };
    // `0` is ignored when a precision is given (C) or when `-` is present.
    assemble(spec, prefix, digits.as_bytes(), spec.precision.is_none())
}

/// `%u` / `%o` / `%x` / `%X` of a `u64` (C `%llu`/`%llo`/`%llx`/`%llX`).
pub(crate) fn format_uint(spec: &FormatSpec, conv: u8, v: u64) -> Vec<u8> {
    let mut digits = match conv {
        b'o' => format!("{:o}", v),
        b'x' => format!("{:x}", v),
        b'X' => format!("{:X}", v),
        _ => format!("{}", v), // b'u'
    };
    apply_int_precision(&mut digits, spec.precision, v == 0);
    if conv == b'o' && spec.alt && !digits.starts_with('0') {
        // `#o` increases the precision just enough to force a leading zero.
        digits.insert(0, '0');
    }
    let prefix: &[u8] = match conv {
        b'x' if spec.alt && v != 0 => b"0x",
        b'X' if spec.alt && v != 0 => b"0X",
        _ => b"",
    };
    assemble(spec, prefix, digits.as_bytes(), spec.precision.is_none())
}

/// `%c` of the argument's low byte (C casts the promoted int to
/// `unsigned char`). Padding is always spaces; the result may be NUL.
pub(crate) fn format_char(spec: &FormatSpec, c: u8) -> Vec<u8> {
    assemble(spec, b"", &[c], false)
}

/// `%s` with width and/or precision over the string's bytes. C's `snprintf`
/// consumes a C string, so formatting stops at the first NUL; the precision
/// then caps the byte count and the width pads with spaces.
pub(crate) fn format_bytes(spec: &FormatSpec, s: &[u8]) -> Vec<u8> {
    let s = match s.iter().position(|&b| b == 0) {
        Some(nul) => &s[..nul],
        None => s,
    };
    let s = match spec.precision {
        Some(p) if p < s.len() => &s[..p],
        _ => s,
    };
    assemble(spec, b"", s, false)
}

/// Fixed-point body (`%f`) of a non-negative finite value.
fn fixed_body(a: f64, precision: usize, alt: bool) -> String {
    let mut s = format!("{:.*}", precision, a);
    if alt && precision == 0 {
        s.push('.');
    }
    s
}

/// Split Rust's `{:e}` output (`mantissa` `e` `exponent`, e.g. `1.50e-3`)
/// into its mantissa and decimal exponent.
fn split_sci(s: &str) -> (&str, i32) {
    let (mant, exp) = s
        .split_once('e')
        .expect("Rust `{:e}` output always contains an exponent");
    (
        mant,
        exp.parse().expect("Rust `{:e}` exponent is an integer"),
    )
}

/// Append a C-style exponent (`e+05`, `E-308`: explicit sign, at least two
/// digits) to `out`.
fn push_exponent(out: &mut String, e_char: char, exp: i32) {
    out.push(e_char);
    out.push(if exp < 0 { '-' } else { '+' });
    let mag = exp.unsigned_abs();
    if mag < 10 {
        out.push('0');
    }
    out.push_str(&format!("{}", mag));
}

/// Scientific body (`%e` / `%E`) of a non-negative finite value.
fn sci_body(a: f64, precision: usize, alt: bool, e_char: char) -> String {
    let formatted = format!("{:.*e}", precision, a);
    let (mant, exp) = split_sci(&formatted);
    let mut out = String::from(mant);
    if alt && precision == 0 {
        out.push('.');
    }
    push_exponent(&mut out, e_char, exp);
    out
}

/// Strip the trailing zeros (and then a trailing point) that `%g` removes.
fn strip_g(s: &mut String) {
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
}

/// General body (`%g` / `%G`) of a non-negative finite value: C99 picks `%e`
/// when the rounded decimal exponent X is `< -4` or `>= precision`, else `%f`
/// with precision `P-1-X`, then drops trailing zeros unless `#` is present.
fn general_body(a: f64, precision: usize, alt: bool, e_char: char) -> String {
    let p = if precision == 0 { 1 } else { precision };
    // The exponent of the value *after* rounding to p significant digits
    // (9.9999e5 at p == 2 must become 1.0e+06, not 10e+05).
    let probe = format!("{:.*e}", p - 1, a);
    let (mant, exp) = split_sci(&probe);
    if exp < -4 || exp >= p as i32 {
        let mut out = String::from(mant);
        if !alt {
            strip_g(&mut out);
        } else if !out.contains('.') {
            out.push('.');
        }
        push_exponent(&mut out, e_char, exp);
        out
    } else {
        let mut out = format!("{:.*}", (p as i32 - 1 - exp) as usize, a);
        if !alt {
            strip_g(&mut out);
        } else if !out.contains('.') {
            out.push('.');
        }
        out
    }
}

/// `%e` / `%E` / `%f` / `%g` / `%G` of an `f64`.
pub(crate) fn format_float(spec: &FormatSpec, conv: u8, v: f64) -> Vec<u8> {
    let upper = conv.is_ascii_uppercase();
    if v.is_nan() {
        // No `-` regardless of the sign bit: freshly produced NaNs carry
        // platform-dependent sign bits (x86 0/0 is negative, wasm's is not),
        // and identical native/wasm output is the whole point.
        let prefix: &[u8] = if spec.plus {
            b"+"
        } else if spec.space {
            b" "
        } else {
            b""
        };
        let body: &[u8] = if upper { b"NAN" } else { b"nan" };
        return assemble(spec, prefix, body, false); // `0` is ignored for nan
    }
    if v.is_infinite() {
        let prefix: &[u8] = if v < 0.0 {
            b"-"
        } else if spec.plus {
            b"+"
        } else if spec.space {
            b" "
        } else {
            b""
        };
        let body: &[u8] = if upper { b"INF" } else { b"inf" };
        return assemble(spec, prefix, body, false); // `0` is ignored for inf
    }

    let precision = spec.precision.unwrap_or(6);
    let a = v.abs();
    let body = match conv {
        b'f' => fixed_body(a, precision, spec.alt),
        b'e' | b'E' => sci_body(a, precision, spec.alt, if upper { 'E' } else { 'e' }),
        _ => general_body(a, precision, spec.alt, if upper { 'E' } else { 'e' }), // b'g' | b'G'
    };
    let prefix: &[u8] = if v.is_sign_negative() {
        b"-"
    } else if spec.plus {
        b"+"
    } else if spec.space {
        b" "
    } else {
        b""
    };
    assemble(spec, prefix, body.as_bytes(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(s: &str) -> FormatSpec {
        parse_format_spec(s.as_bytes())
    }

    fn int(form: &str, v: i64) -> String {
        String::from_utf8(format_int(&spec(form), v)).unwrap()
    }

    fn uint(form: &str, conv: u8, v: u64) -> String {
        String::from_utf8(format_uint(&spec(form), conv, v)).unwrap()
    }

    fn float(form: &str, conv: u8, v: f64) -> String {
        String::from_utf8(format_float(&spec(form), conv, v)).unwrap()
    }

    #[test]
    fn spec_parsing() {
        let fs = spec("-+ #012.34");
        assert!(fs.left && fs.plus && fs.space && fs.alt && fs.zero);
        assert_eq!(fs.width, 12);
        assert_eq!(fs.precision, Some(34));
        let fs = spec("");
        assert!(!fs.left && !fs.plus && !fs.space && !fs.alt && !fs.zero);
        assert_eq!(fs.width, 0);
        assert_eq!(fs.precision, None);
        // A bare `.` is precision zero; a leading `0` is a flag, not width.
        assert_eq!(spec(".").precision, Some(0));
        let fs = spec("05");
        assert!(fs.zero);
        assert_eq!(fs.width, 5);
    }

    #[test]
    fn decimal_integers() {
        assert_eq!(int("", 42), "42");
        assert_eq!(int("", -42), "-42");
        assert_eq!(int("", 0), "0");
        assert_eq!(int("5", 42), "   42");
        assert_eq!(int("-5", 42), "42   ");
        assert_eq!(int("05", 42), "00042");
        assert_eq!(int("05", -42), "-0042");
        assert_eq!(int("+", 42), "+42");
        assert_eq!(int("+", -42), "-42");
        assert_eq!(int(" ", 42), " 42");
        assert_eq!(int(".5", 42), "00042");
        assert_eq!(int("8.5", 42), "   00042");
        assert_eq!(int("-8.5", -42), "-00042  ");
        assert_eq!(int("08.5", 42), "   00042"); // `0` ignored with precision
        assert_eq!(int(".0", 0), "");
        assert_eq!(int("5.0", 0), "     ");
        assert_eq!(int("", i64::MAX), "9223372036854775807");
        assert_eq!(int("", i64::MIN), "-9223372036854775808");
        assert_eq!(int("010", 23), "0000000023");
    }

    #[test]
    fn unsigned_integers() {
        assert_eq!(uint("", b'u', u64::MAX), "18446744073709551615");
        assert_eq!(uint("", b'x', u64::MAX), "ffffffffffffffff");
        assert_eq!(uint("", b'X', u64::MAX), "FFFFFFFFFFFFFFFF");
        assert_eq!(uint("", b'o', u64::MAX), "1777777777777777777777");
        assert_eq!(uint("#", b'x', 255), "0xff");
        assert_eq!(uint("#", b'X', 255), "0XFF");
        assert_eq!(uint("#010", b'x', 255), "0x000000ff");
        assert_eq!(uint("#", b'x', 0), "0"); // no 0x prefix for zero
        assert_eq!(uint("#.0", b'x', 0), ""); // nothing at all
        assert_eq!(uint("#", b'o', 8), "010");
        assert_eq!(uint("#", b'o', 0), "0"); // exactly one zero
        assert_eq!(uint("#.0", b'o', 0), "0"); // alt forces the zero back
        assert_eq!(uint("#.5", b'o', 8), "00010"); // precision already leads 0
        assert_eq!(uint(".0", b'u', 0), "");
        assert_eq!(uint("8", b'x', 255), "      ff");
        assert_eq!(uint("-8", b'x', 255), "ff      ");
        assert_eq!(uint("08", b'x', 255), "000000ff");
    }

    #[test]
    fn chars_and_strings() {
        let fs = spec("");
        assert_eq!(format_char(&fs, b'A'), b"A");
        assert_eq!(format_char(&spec("5"), b'A'), b"    A");
        assert_eq!(format_char(&spec("-5"), b'A'), b"A    ");
        assert_eq!(format_char(&spec("5"), 0), b"    \0");
        assert_eq!(format_bytes(&spec("5"), b"ab"), b"   ab");
        assert_eq!(format_bytes(&spec("-5"), b"ab"), b"ab   ");
        assert_eq!(format_bytes(&spec(".1"), b"abc"), b"a");
        assert_eq!(format_bytes(&spec("5.2"), b"abc"), b"   ab");
        assert_eq!(format_bytes(&spec(".0"), b"abc"), b"");
        // C-string semantics: an embedded NUL terminates the value.
        assert_eq!(format_bytes(&spec("5"), b"a\0b"), b"    a");
    }

    #[test]
    fn fixed_floats() {
        assert_eq!(float("", b'f', 0.0), "0.000000");
        assert_eq!(float("", b'f', -0.0), "-0.000000");
        assert_eq!(float("", b'f', 10.3), "10.300000");
        assert_eq!(float(".2", b'f', 1.5), "1.50");
        assert_eq!(float(".0", b'f', 2.5), "2"); // round-half-even
        assert_eq!(float(".0", b'f', 3.5), "4");
        assert_eq!(float("#.0", b'f', 5.0), "5.");
        assert_eq!(float("010.2", b'f', -1.5), "-000001.50");
        assert_eq!(float("+.1", b'f', 1.25), "+1.2");
        assert_eq!(float(" .1", b'f', 1.25), " 1.2");
        assert_eq!(float("-8.2", b'f', 1.5), "1.50    ");
        // the longest number that can be formatted (conformance: >= 100 chars)
        assert!(float("99.99", b'f', -1e308).len() >= 100);
    }

    #[test]
    fn scientific_floats() {
        assert_eq!(float("", b'e', 1.5), "1.500000e+00");
        assert_eq!(float("", b'E', -1.5), "-1.500000E+00");
        assert_eq!(float(".0", b'e', 12345.0), "1e+04");
        assert_eq!(float("#.0", b'e', 5.0), "5.e+00");
        assert_eq!(float("", b'e', 0.0), "0.000000e+00");
        assert_eq!(float(".2", b'e', 9.999), "1.00e+01"); // rounding carries
        assert_eq!(float(".2", b'e', 0.000123), "1.23e-04");
        assert_eq!(float(".2", b'e', 1e308), "1.00e+308");
        assert_eq!(float("012.2", b'e', -1.5), "-0001.50e+00");
    }

    #[test]
    fn general_floats() {
        assert_eq!(float("", b'g', 100000.0), "100000");
        assert_eq!(float("", b'g', 1000000.0), "1e+06");
        assert_eq!(float("", b'g', 0.0001), "0.0001");
        assert_eq!(float("", b'g', 0.00001), "1e-05");
        assert_eq!(float("", b'g', 0.0), "0");
        assert_eq!(float("", b'g', 0.5), "0.5");
        assert_eq!(float(".3", b'g', 1234.5), "1.23e+03");
        assert_eq!(float(".0", b'g', 1234.5), "1e+03"); // precision 0 acts as 1
        assert_eq!(float("#", b'g', 1.0), "1.00000"); // `#` keeps the zeros
        assert_eq!(float("#.1", b'g', 5.0), "5.");
        assert_eq!(float("", b'G', 1e-10), "1E-10");
        assert_eq!(float("", b'g', 999999.5), "1e+06"); // rounding crosses P
                                                        // `#` keeps the zeros even across the decade crossover (C99; BSD libc
                                                        // agrees, glibc strips them to `1.e+06` — see floats_match_snprintf).
        assert_eq!(float("#", b'g', 999999.5), "1.00000e+06");
        assert_eq!(float("#", b'G', 999999.5), "1.00000E+06");
        assert_eq!(float(".17", b'g', 0.1), "0.10000000000000001");
    }

    #[test]
    fn nonfinite_floats() {
        assert_eq!(float("", b'f', f64::INFINITY), "inf");
        assert_eq!(float("", b'f', f64::NEG_INFINITY), "-inf");
        assert_eq!(float("+", b'f', f64::INFINITY), "+inf");
        assert_eq!(float("", b'E', f64::INFINITY), "INF");
        assert_eq!(float("", b'G', f64::INFINITY), "INF");
        assert_eq!(float("8", b'f', f64::INFINITY), "     inf");
        assert_eq!(float("-8", b'f', f64::INFINITY), "inf     ");
        assert_eq!(float("08", b'f', f64::INFINITY), "     inf"); // `0` ignored
        assert_eq!(float("", b'f', f64::NAN), "nan");
        assert_eq!(float("", b'f', -f64::NAN), "nan"); // sign bit ignored
        assert_eq!(float("+", b'f', f64::NAN), "+nan");
        assert_eq!(float("", b'G', f64::NAN), "NAN");
        // precision is irrelevant for non-finite values
        assert_eq!(float(".3", b'f', f64::INFINITY), "inf");
    }

    /// Cross-check the pure-Rust formatter against the platform's C
    /// `snprintf` — the exact code path native `string.format` used before —
    /// over a matrix of C-defined flag/width/precision/value combinations.
    /// (Non-finite values are excluded: their sign handling is the one
    /// deliberate, documented divergence. C-undefined combinations such as
    /// `+` on `%u` or `0` on `%s` are excluded too.)
    #[cfg(not(target_arch = "wasm32"))]
    mod c_oracle {
        use super::*;
        use core::ffi::{c_char, c_int};

        extern "C" {
            fn snprintf(s: *mut c_char, n: usize, format: *const c_char, ...) -> c_int;
        }

        const WIDTHS: [&str; 4] = ["", "1", "8", "20"];

        fn c_form(spec: &str, length_mod: &str, conv: char) -> Vec<u8> {
            let mut f = format!("%{spec}{length_mod}{conv}").into_bytes();
            f.push(0);
            f
        }

        fn c_call_bytes(fill: impl FnOnce(&mut [u8]) -> c_int) -> Vec<u8> {
            let mut buf = [0u8; 2048];
            let n = fill(&mut buf);
            assert!(n >= 0 && (n as usize) < buf.len());
            buf[..n as usize].to_vec()
        }

        fn c_call(fill: impl FnOnce(&mut [u8]) -> c_int) -> String {
            String::from_utf8(c_call_bytes(fill)).unwrap()
        }

        #[test]
        fn signed_matches_snprintf() {
            let values: [i64; 8] = [0, 1, -1, 42, -42, 12345678, i64::MAX, i64::MIN];
            for flags in ["", "-", "+", " ", "0", "-+", "+ ", "0+", "- ", "0 "] {
                for width in WIDTHS {
                    for prec in ["", ".0", ".5", ".20"] {
                        for v in values {
                            let s = format!("{flags}{width}{prec}");
                            let form = c_form(&s, "ll", 'd');
                            let expect = c_call(|buf| unsafe {
                                snprintf(
                                    buf.as_mut_ptr() as *mut c_char,
                                    buf.len(),
                                    form.as_ptr() as *const c_char,
                                    v,
                                )
                            });
                            assert_eq!(int(&s, v), expect, "%{s}d of {v}");
                        }
                    }
                }
            }
        }

        #[test]
        fn unsigned_matches_snprintf() {
            let values: [u64; 7] = [0, 1, 8, 255, 4096, u64::MAX, i64::MIN as u64];
            for conv in ['u', 'o', 'x', 'X'] {
                let flag_sets: &[&str] = if conv == 'u' {
                    &["", "-", "0"]
                } else {
                    &["", "-", "#", "0", "#0", "-#"]
                };
                for flags in flag_sets {
                    for width in WIDTHS {
                        for prec in ["", ".0", ".5", ".20"] {
                            for v in values {
                                let s = format!("{flags}{width}{prec}");
                                let form = c_form(&s, "ll", conv);
                                let expect = c_call(|buf| unsafe {
                                    snprintf(
                                        buf.as_mut_ptr() as *mut c_char,
                                        buf.len(),
                                        form.as_ptr() as *const c_char,
                                        v,
                                    )
                                });
                                assert_eq!(uint(&s, conv as u8, v), expect, "%{s}{conv} of {v}");
                            }
                        }
                    }
                }
            }
        }

        #[test]
        fn floats_match_snprintf() {
            let values: [f64; 16] = [
                0.0,
                -0.0,
                0.1,
                0.5,
                1.0,
                -1.5,
                2.5,
                core::f64::consts::PI,
                1e-9,
                12345.6789,
                999999.5,
                9.999999e5,
                1e20,
                1.7976931348623157e308,
                2.2250738585072014e-308,
                5e-324,
            ];
            for conv in ['e', 'E', 'f', 'g', 'G'] {
                // glibc strips the trailing zeros `#` is supposed to keep
                // (C99 7.19.6.1) from `%#g`/`%#G` when rounding crosses into
                // the next decade: `%#g` of 999999.5 prints `1.e+06`, not the
                // `1.00000e+06` BSD libc (and this formatter) produce. Skip
                // `#` for g/G here; `general_floats` pins the C99 behaviour.
                let flag_sets: &[&str] = if conv == 'g' || conv == 'G' {
                    &["", "-", "+", " ", "0", "+0", " 0"]
                } else {
                    &["", "-", "+", " ", "#", "0", "+0", "#0", "-#", " 0", "-+ #0"]
                };
                for flags in flag_sets {
                    for width in WIDTHS {
                        for prec in ["", ".0", ".1", ".6", ".17"] {
                            for v in values {
                                let s = format!("{flags}{width}{prec}");
                                let form = c_form(&s, "", conv);
                                let expect = c_call(|buf| unsafe {
                                    snprintf(
                                        buf.as_mut_ptr() as *mut c_char,
                                        buf.len(),
                                        form.as_ptr() as *const c_char,
                                        v,
                                    )
                                });
                                assert_eq!(float(&s, conv as u8, v), expect, "%{s}{conv} of {v}");
                            }
                        }
                    }
                }
            }
        }

        #[test]
        fn chars_match_snprintf() {
            for flags in ["", "-"] {
                for width in WIDTHS {
                    for v in [1u8, 65, 128, 255] {
                        let s = format!("{flags}{width}");
                        let form = c_form(&s, "", 'c');
                        let expect = c_call_bytes(|buf| unsafe {
                            snprintf(
                                buf.as_mut_ptr() as *mut c_char,
                                buf.len(),
                                form.as_ptr() as *const c_char,
                                v as c_int,
                            )
                        });
                        assert_eq!(format_char(&spec(&s), v), expect, "%{s}c of {v}");
                    }
                }
            }
        }

        #[test]
        fn strings_match_snprintf() {
            for flags in ["", "-"] {
                for width in WIDTHS {
                    for prec in ["", ".0", ".2", ".8"] {
                        for v in ["", "a", "hello", "hello world, longer"] {
                            let s = format!("{flags}{width}{prec}");
                            let form = c_form(&s, "", 's');
                            let mut cv = v.as_bytes().to_vec();
                            cv.push(0);
                            let expect = c_call(|buf| unsafe {
                                snprintf(
                                    buf.as_mut_ptr() as *mut c_char,
                                    buf.len(),
                                    form.as_ptr() as *const c_char,
                                    cv.as_ptr() as *const c_char,
                                )
                            });
                            assert_eq!(
                                String::from_utf8(format_bytes(&spec(&s), v.as_bytes())).unwrap(),
                                expect,
                                "%{s}s of {v:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
