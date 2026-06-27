//! Minimal libc surface for the `wasm32-unknown-unknown` target.
//!
//! `wasm32-unknown-unknown` ships no libc, so the handful of C functions the
//! faithful translation declares via `extern "C"` (the VM's C-style allocator,
//! `string.format`'s `snprintf`, number parsing's `strtoull`, etc.) have no
//! symbol to bind to and would otherwise surface as unresolved `env` imports in
//! the generated wasm.
//!
//! Rather than require the browser host to supply a libc, these provide the
//! small subset that the run / type-check paths actually exercise, backed by
//! Rust's own global allocator and `core`. They are compiled **only** for wasm
//! (`#[cfg(target_arch = "wasm32")]`); the native build is completely
//! unaffected and continues to bind the platform libc.
//!
//! The allocator uses a size-prefixed block layout so that a bare `free(ptr)` /
//! `realloc(ptr, n)` (which carry no size) can recover the original allocation
//! size: each block reserves a `usize`-aligned header storing the user size,
//! and the returned pointer points just past it.

#![cfg(target_arch = "wasm32")]

use core::ffi::{c_char, c_int, c_void};

/// Bytes reserved before every user pointer to record the block's user size.
/// Sized/aligned to the maximum alignment we hand out (16 bytes) so the user
/// region is suitably aligned for any Luau value.
const HEADER: usize = 16;

/// `malloc(size)` — allocate `size` bytes (size-prefixed).
#[cfg(target_os = "unknown")] // wasi-libc provides malloc/free/realloc
#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if size == 0 {
        // Return a unique, non-null, never-dereferenced pointer (C `malloc(0)`
        // is allowed to return either NULL or a freeable unique pointer; the
        // callers treat NULL as failure, so hand out a real 1-byte block).
        return malloc(1);
    }
    let total = size + HEADER;
    let layout = match core::alloc::Layout::from_size_align(total, HEADER) {
        Ok(l) => l,
        Err(_) => return core::ptr::null_mut(),
    };
    let base = alloc::alloc::alloc(layout);
    if base.is_null() {
        return core::ptr::null_mut();
    }
    *(base as *mut usize) = size;
    base.add(HEADER) as *mut c_void
}

/// `free(ptr)` — release a block previously returned by [`malloc`]/[`realloc`].
#[cfg(target_os = "unknown")]
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let base = (ptr as *mut u8).sub(HEADER);
    let size = *(base as *mut usize);
    let layout = core::alloc::Layout::from_size_align_unchecked(size + HEADER, HEADER);
    alloc::alloc::dealloc(base, layout);
}

/// `realloc(ptr, size)` — resize a block, preserving its contents.
#[cfg(target_os = "unknown")]
#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        free(ptr);
        return core::ptr::null_mut();
    }
    let base = (ptr as *mut u8).sub(HEADER);
    let old_size = *(base as *mut usize);
    let old_layout = core::alloc::Layout::from_size_align_unchecked(old_size + HEADER, HEADER);
    let new_base = alloc::alloc::realloc(base, old_layout, size + HEADER);
    if new_base.is_null() {
        return core::ptr::null_mut();
    }
    *(new_base as *mut usize) = size;
    new_base.add(HEADER) as *mut c_void
}

/// `strchr(s, c)` — first occurrence of `c` in the NUL-terminated `s`, or NULL.
/// Matching the terminating NUL when `c == 0` is part of the C contract.
#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const c_char, c: c_int) -> *mut c_char {
    let target = c as u8 as c_char;
    let mut p = s;
    loop {
        let ch = *p;
        if ch == target {
            return p as *mut c_char;
        }
        if ch == 0 {
            return core::ptr::null_mut();
        }
        p = p.add(1);
    }
}

/// `time(t)` — seconds since the Unix epoch. `wasm32-unknown-unknown` has no
/// clock syscall; rather than pull a JS host clock in as another import, this
/// reports a fixed reference instant. `os.time` / `os.date` therefore return a
/// stable value in the browser, which is fine for a playground (the scripts
/// shown do not depend on the wall clock). The result is also written through
/// `t` when non-null, matching the C signature.
#[no_mangle]
pub unsafe extern "C" fn time(t: *mut i64) -> i64 {
    // 2024-01-01T00:00:00Z — a fixed, deterministic reference instant.
    const REFERENCE_EPOCH: i64 = 1_704_067_200;
    if !t.is_null() {
        *t = REFERENCE_EPOCH;
    }
    REFERENCE_EPOCH
}

/// `clock()` — processor time in `CLOCKS_PER_SEC` units. With no wasm clock
/// syscall available without an extra host import, this reports zero; `os.clock`
/// is only used for timing and is not load-bearing for the playground.
#[no_mangle]
pub unsafe extern "C" fn clock() -> core::ffi::c_long {
    0
}

/// `sysconf(name)` — only the page-size query is ever issued (by the JIT code
/// allocator, which is not executed in the interpreter path). Report 64 KiB,
/// the wasm page size.
#[no_mangle]
pub unsafe extern "C" fn sysconf(_name: c_int) -> core::ffi::c_long {
    65536
}

/// `strtoul(nptr, endptr, base)` — parse an unsigned long from a C string in the
/// given radix, used by Luau's number lexing on the wasm path (where there is no
/// host libc to bind). Mirrors C semantics: skip leading whitespace, accept an
/// optional sign, honour a `0x`/`0` prefix for base 0/16, stop at the first
/// non-digit, and write that position to `*endptr`. Overflow saturates to
/// `c_ulong::MAX` (the C `ERANGE` return). `base` is assumed in `0..=36`.
#[no_mangle]
pub unsafe extern "C" fn strtoul(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> core::ffi::c_ulong {
    use core::ffi::c_ulong;

    if nptr.is_null() {
        if !endptr.is_null() {
            *endptr = nptr as *mut c_char;
        }
        return 0;
    }

    let mut p = nptr;
    // Leading whitespace (space, \t, \n, \v, \f, \r).
    while matches!(*p as u8, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        p = p.add(1);
    }

    // Optional sign — C strtoul accepts '-' and returns the negation (mod 2^N).
    let mut neg = false;
    match *p as u8 {
        b'+' => p = p.add(1),
        b'-' => {
            neg = true;
            p = p.add(1);
        }
        _ => {}
    }

    // Base 0 auto-detection and the optional 0x / 0X prefix for hex.
    let mut base = base;
    if (base == 0 || base == 16) && *p as u8 == b'0' && matches!(*p.add(1) as u8, b'x' | b'X') {
        p = p.add(2);
        base = 16;
    } else if base == 0 {
        base = if *p as u8 == b'0' { 8 } else { 10 };
    }
    let radix = base as c_ulong;

    let mut acc: c_ulong = 0;
    let mut any = false;
    let mut overflow = false;
    loop {
        let c = *p as u8;
        let digit = match c {
            b'0'..=b'9' => (c - b'0') as c_ulong,
            b'a'..=b'z' => (c - b'a' + 10) as c_ulong,
            b'A'..=b'Z' => (c - b'A' + 10) as c_ulong,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        let (mul, o1) = acc.overflowing_mul(radix);
        let (add, o2) = mul.overflowing_add(digit);
        if o1 || o2 {
            overflow = true;
        }
        acc = add;
        any = true;
        p = p.add(1);
    }

    if !endptr.is_null() {
        // No digits consumed ⇒ point back at the original start, per C.
        *endptr = (if any { p } else { nptr }) as *mut c_char;
    }
    if overflow {
        return c_ulong::MAX;
    }
    if neg {
        acc.wrapping_neg()
    } else {
        acc
    }
}

// The page-mapping family is referenced only by the native-codegen page
// allocator, which is never reached on the interpreter-only wasm path. They are
// provided as faithful failures (mmap returns MAP_FAILED; the rest no-op) so
// the module links without a JS host having to supply them.

/// `mmap` — JIT page allocation; unreachable on the wasm interpreter path.
#[no_mangle]
pub unsafe extern "C" fn mmap(
    _addr: *mut c_void,
    _len: usize,
    _prot: c_int,
    _flags: c_int,
    _fd: c_int,
    _off: i64,
) -> *mut c_void {
    // MAP_FAILED == (void*)-1
    usize::MAX as *mut c_void
}

/// `munmap` — JIT page release; unreachable on the wasm interpreter path.
#[no_mangle]
pub unsafe extern "C" fn munmap(_addr: *mut c_void, _len: usize) -> c_int {
    0
}

/// `mprotect` — JIT page permissions; unreachable on the wasm interpreter path.
#[no_mangle]
pub unsafe extern "C" fn mprotect(_addr: *mut c_void, _len: usize, _prot: c_int) -> c_int {
    0
}

/// `strtoull(nptr, endptr, base)` — parse an unsigned long long, supporting the
/// bases Luau's number lexer asks for (10 and 16, plus the C `base == 0`
/// auto-detect of a `0x`/`0X` hex prefix). Leading ASCII whitespace and an
/// optional `+`/`-` sign are skipped, mirroring C. `*endptr`, when non-null, is
/// set past the last consumed digit.
#[no_mangle]
pub unsafe extern "C" fn strtoull(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    mut base: c_int,
) -> u64 {
    let mut p = nptr as *const u8;

    // Skip leading whitespace (space, \t, \n, \v, \f, \r).
    while matches!(*p, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        p = p.add(1);
    }

    // Optional sign. A leading '-' negates modulo 2^64, like C.
    let mut negate = false;
    if *p == b'+' {
        p = p.add(1);
    } else if *p == b'-' {
        negate = true;
        p = p.add(1);
    }

    // Hex prefix handling for base 0 / 16.
    if (base == 0 || base == 16) && *p == b'0' && matches!(*p.add(1), b'x' | b'X') {
        p = p.add(2);
        base = 16;
    } else if base == 0 {
        base = 10;
    }

    let radix = base as u64;
    let mut acc: u64 = 0;
    let start = p;
    loop {
        let digit = match *p {
            c @ b'0'..=b'9' => (c - b'0') as u64,
            c @ b'a'..=b'z' => (c - b'a' + 10) as u64,
            c @ b'A'..=b'Z' => (c - b'A' + 10) as u64,
            _ => break,
        };
        if digit >= radix {
            break;
        }
        acc = acc.wrapping_mul(radix).wrapping_add(digit);
        p = p.add(1);
    }

    if !endptr.is_null() {
        // If no digits were consumed, C points endptr at the original string.
        let end = if p == start { nptr } else { p as *const c_char };
        *endptr = end as *mut c_char;
    }

    if negate {
        acc.wrapping_neg()
    } else {
        acc
    }
}

/// Broken-down time, matching the C `struct tm` field layout/order the
/// translation's `os_date` uses.
#[repr(C)]
pub struct Tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
}

/// Convert a Unix timestamp to broken-down UTC time (civil calendar), shared by
/// [`gmtime_r`] and [`localtime_r`] (the wasm build has no local timezone, so
/// local == UTC). Uses Howard Hinnant's well-known days-from-civil inverse.
unsafe fn fill_tm(secs: i64, result: *mut Tm) {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);

    (*result).tm_hour = (rem / 3600) as c_int;
    (*result).tm_min = ((rem % 3600) / 60) as c_int;
    (*result).tm_sec = (rem % 60) as c_int;

    // 1970-01-01 was a Thursday (wday 4).
    (*result).tm_wday = (((days % 7) + 4 + 7) % 7) as c_int;

    // days -> civil (y, m, d), m in [1,12], d in [1,31].
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    (*result).tm_year = (year - 1900) as c_int;
    (*result).tm_mon = (m - 1) as c_int;
    (*result).tm_mday = d as c_int;

    // tm_yday: days since Jan 1 of tm_year.
    let jan1 = days_from_civil(year, 1, 1);
    (*result).tm_yday = (days - jan1) as c_int;
    (*result).tm_isdst = 0;
}

/// Days from 1970-01-01 to the given civil date (Hinnant's days_from_civil).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `gmtime_r(timep, result)` — UTC broken-down time.
#[no_mangle]
pub unsafe extern "C" fn gmtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm {
    if timep.is_null() || result.is_null() {
        return core::ptr::null_mut();
    }
    fill_tm(*timep, result);
    result
}

/// `localtime_r(timep, result)` — local broken-down time. The wasm build has no
/// timezone database, so local time is UTC.
#[no_mangle]
pub unsafe extern "C" fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm {
    gmtime_r(timep, result)
}
