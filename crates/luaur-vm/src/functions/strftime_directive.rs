//! Pure-Rust rendering of a single `os.date` conversion specifier.
//!
//! The faithful translation of `os_date` rendered each validated `%x`
//! specifier through C `strftime`. Native builds bind libc, but
//! `wasm32-unknown-unknown` ships none, so `strftime` was an unbound `env`
//! import and any `os.date("%H:%M")`-style call trapped with `unreachable` in
//! the browser (the same defect class as the former `string.format` /
//! `snprintf` dependency). This renders the full set Luau validates against
//! (`LUA_STRFTIMEOPTIONS` = `aAbBcdHIjmMpSUwWxXyYzZ%`) in portable Rust, with
//! C-locale strings — which is what libc produced anyway, since the process
//! never calls `setlocale`.
//!
//! Timezone policy:
//! - The broken-down `tm` is still produced by the platform (`gmtime_r` /
//!   `localtime_r` from libc on native; the pure-Rust shims in
//!   `luaur-common::wasm_libc` on wasm, where there is no TZ database and
//!   local time **is** UTC by definition of those shims).
//! - On non-Windows targets `%z` / `%Z` render from the `tm_gmtoff` /
//!   `tm_zone` fields — exactly the fields libc `strftime` itself reads — so
//!   native output is unchanged and the wasm shims pin them deterministically
//!   (`+0000` / `UTC`).
//! - The MSVC `tm` carries no offset/zone fields, so on Windows `%z` / `%Z`
//!   render no characters, the C89-sanctioned result when the timezone is not
//!   determinable from the argument. (Previously MSVC's `strftime` substituted
//!   the localized zone *name* for both — data the portable `tm` simply does
//!   not carry.)
//!
//! Out-of-range `tm` fields cannot occur through `os.date` (the struct always
//! comes from `gmtime_r`/`localtime_r`), but the arithmetic is widened and the
//! table lookups wrapped so hostile values can never panic the VM.

use crate::functions::localtime_r::tm;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;

const WEEKDAY_ABBR: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAY_FULL: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTH_FULL: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// C `%U` / `%W`: full weeks since the first Sunday (resp. Monday) of the
/// year, with the days before it in week 0.
fn week_number(yday: core::ffi::c_int, wday: core::ffi::c_int, monday_first: bool) -> i64 {
    let yday = (yday as i64).max(0);
    let wday = (wday as i64).rem_euclid(7);
    let wday = if monday_first { (wday + 6) % 7 } else { wday };
    (yday + 7 - wday) / 7
}

/// `%z`: the UTC offset carried by the `tm` itself (what libc `strftime`
/// reads), `±hhmm`. The MSVC `tm` has no such field, so Windows renders no
/// characters (C89: "no characters if no time zone is determinable").
fn utc_offset(t: &tm) -> String {
    #[cfg(not(target_os = "windows"))]
    {
        let off = t.tm_gmtoff as i64;
        let (sign, off) = if off < 0 { ('-', -off) } else { ('+', off) };
        format!("{}{:02}{:02}", sign, off / 3600, (off % 3600) / 60)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = t;
        String::new()
    }
}

/// `%Z`: the zone abbreviation carried by the `tm` itself (what libc
/// `strftime` reads), or no characters when the `tm` has none.
fn zone_name(t: &tm) -> String {
    #[cfg(not(target_os = "windows"))]
    {
        if t.tm_zone.is_null() {
            return String::new();
        }
        // Trusted exactly as far as libc strftime trusted it: tm_zone is set
        // by gmtime_r/localtime_r (static libc storage) or by the wasm shim
        // (a &'static CStr).
        let mut out = String::new();
        let mut p = t.tm_zone;
        unsafe {
            while *p != 0 {
                out.push(*p as u8 as char);
                p = p.add(1);
            }
        }
        out
    }
    #[cfg(target_os = "windows")]
    {
        let _ = t;
        String::new()
    }
}

/// Render one `os.date` conversion specifier (already validated against
/// `LUA_STRFTIMEOPTIONS`) from a broken-down `tm`, byte-identical to what the
/// C-locale libc `strftime` produced on native.
pub(crate) fn strftime_directive(t: &tm, conv: u8) -> String {
    let wday = (t.tm_wday as i64).rem_euclid(7) as usize;
    let mon = (t.tm_mon as i64).rem_euclid(12) as usize;
    let year = t.tm_year as i64 + 1900;
    let hour = (t.tm_hour as i64).rem_euclid(24);
    match conv {
        b'a' => WEEKDAY_ABBR[wday].to_string(),
        b'A' => WEEKDAY_FULL[wday].to_string(),
        b'b' => MONTH_ABBR[mon].to_string(),
        b'B' => MONTH_FULL[mon].to_string(),
        // C-locale `%c` is `%a %b %e %H:%M:%S %Y` (day of month space-padded).
        b'c' => format!(
            "{} {} {:2} {:02}:{:02}:{:02} {}",
            WEEKDAY_ABBR[wday], MONTH_ABBR[mon], t.tm_mday, t.tm_hour, t.tm_min, t.tm_sec, year
        ),
        b'd' => format!("{:02}", t.tm_mday),
        b'H' => format!("{:02}", t.tm_hour),
        b'I' => format!("{:02}", if hour % 12 == 0 { 12 } else { hour % 12 }),
        b'j' => format!("{:03}", t.tm_yday as i64 + 1),
        b'm' => format!("{:02}", t.tm_mon as i64 + 1),
        b'M' => format!("{:02}", t.tm_min),
        b'p' => (if hour < 12 { "AM" } else { "PM" }).to_string(),
        b'S' => format!("{:02}", t.tm_sec),
        b'U' => format!("{:02}", week_number(t.tm_yday, t.tm_wday, false)),
        b'w' => format!("{}", wday),
        b'W' => format!("{:02}", week_number(t.tm_yday, t.tm_wday, true)),
        // C-locale `%x` is `%m/%d/%y`, `%X` is `%H:%M:%S`.
        b'x' => format!(
            "{:02}/{:02}/{:02}",
            t.tm_mon as i64 + 1,
            t.tm_mday,
            year.rem_euclid(100)
        ),
        b'X' => format!("{:02}:{:02}:{:02}", t.tm_hour, t.tm_min, t.tm_sec),
        b'y' => format!("{:02}", year.rem_euclid(100)),
        b'Y' => format!("{}", year),
        b'z' => utc_offset(t),
        b'Z' => zone_name(t),
        b'%' => "%".to_string(),
        // Unreachable: os_date validates against LUA_STRFTIMEOPTIONS first.
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a UTC `tm` from a unix timestamp (Hinnant's civil-from-days),
    /// mirroring what `gmtime_r` produces, with no zone information attached.
    fn utc_tm(secs: i64) -> tm {
        let days = secs.div_euclid(86_400);
        let rem = secs.rem_euclid(86_400);

        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = if m <= 2 { y + 1 } else { y };

        let jan1_z = {
            // days_from_civil(year, 1, 1): January shifts to the previous
            // Hinnant year (m <= 2), giving doy = 306.
            let yy = year - 1;
            let era = if yy >= 0 { yy } else { yy - 399 } / 400;
            let yoe = yy - era * 400;
            let doe = yoe * 365 + yoe / 4 - yoe / 100 + 306;
            era * 146_097 + doe - 719_468
        };

        #[allow(clippy::needless_update)]
        tm {
            tm_sec: (rem % 60) as core::ffi::c_int,
            tm_min: ((rem % 3600) / 60) as core::ffi::c_int,
            tm_hour: (rem / 3600) as core::ffi::c_int,
            tm_mday: d as core::ffi::c_int,
            tm_mon: (m - 1) as core::ffi::c_int,
            tm_year: (year - 1900) as core::ffi::c_int,
            tm_wday: (((days % 7) + 4 + 7) % 7) as core::ffi::c_int,
            tm_yday: (days - jan1_z) as core::ffi::c_int,
            tm_isdst: 0,
            ..unsafe { core::mem::zeroed() }
        }
    }

    /// unix-from-civil (UTC), for readable test timestamps.
    fn unix(y: i64, m: i64, d: i64, hh: i64, mm: i64, ss: i64) -> i64 {
        let yy = if m <= 2 { y - 1 } else { y };
        let era = if yy >= 0 { yy } else { yy - 399 } / 400;
        let yoe = yy - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        (era * 146_097 + doe - 719_468) * 86_400 + hh * 3600 + mm * 60 + ss
    }

    fn fmt(secs: i64, conv: u8) -> String {
        strftime_directive(&utc_tm(secs), conv)
    }

    #[test]
    fn epoch_start() {
        // Thursday 1970-01-01 00:00:00 UTC
        assert_eq!(fmt(0, b'a'), "Thu");
        assert_eq!(fmt(0, b'A'), "Thursday");
        assert_eq!(fmt(0, b'b'), "Jan");
        assert_eq!(fmt(0, b'B'), "January");
        assert_eq!(fmt(0, b'c'), "Thu Jan  1 00:00:00 1970");
        assert_eq!(fmt(0, b'd'), "01");
        assert_eq!(fmt(0, b'H'), "00");
        assert_eq!(fmt(0, b'I'), "12");
        assert_eq!(fmt(0, b'j'), "001");
        assert_eq!(fmt(0, b'm'), "01");
        assert_eq!(fmt(0, b'M'), "00");
        assert_eq!(fmt(0, b'p'), "AM");
        assert_eq!(fmt(0, b'S'), "00");
        assert_eq!(fmt(0, b'U'), "00");
        assert_eq!(fmt(0, b'w'), "4");
        assert_eq!(fmt(0, b'W'), "00");
        assert_eq!(fmt(0, b'x'), "01/01/70");
        assert_eq!(fmt(0, b'X'), "00:00:00");
        assert_eq!(fmt(0, b'y'), "70");
        assert_eq!(fmt(0, b'Y'), "1970");
        assert_eq!(fmt(0, b'%'), "%");
    }

    #[test]
    fn leap_day_afternoon() {
        let t = unix(2024, 2, 29, 13, 45, 56);
        assert_eq!(fmt(t, b'Y'), "2024");
        assert_eq!(fmt(t, b'm'), "02");
        assert_eq!(fmt(t, b'd'), "29");
        assert_eq!(fmt(t, b'j'), "060");
        assert_eq!(fmt(t, b'a'), "Thu");
        assert_eq!(fmt(t, b'H'), "13");
        assert_eq!(fmt(t, b'I'), "01");
        assert_eq!(fmt(t, b'p'), "PM");
        assert_eq!(fmt(t, b'c'), "Thu Feb 29 13:45:56 2024");
        assert_eq!(fmt(t, b'x'), "02/29/24");
        assert_eq!(fmt(t, b'X'), "13:45:56");
    }

    #[test]
    fn year_boundaries() {
        // 2020 is a leap year: Dec 31 is day 366, a Thursday.
        let t = unix(2020, 12, 31, 23, 59, 59);
        assert_eq!(fmt(t, b'j'), "366");
        assert_eq!(fmt(t, b'a'), "Thu");
        assert_eq!(fmt(t, b'X'), "23:59:59");
        // One second later: Friday 2021-01-01.
        assert_eq!(fmt(t + 1, b'j'), "001");
        assert_eq!(fmt(t + 1, b'a'), "Fri");
        assert_eq!(fmt(t + 1, b'Y'), "2021");
        // Pre-epoch (the `!` UTC path allows negative timestamps).
        assert_eq!(fmt(-1, b'Y'), "1969");
        assert_eq!(fmt(-1, b'y'), "69");
        assert_eq!(fmt(-1, b'X'), "23:59:59");
        assert_eq!(fmt(-1, b'j'), "365");
    }

    #[test]
    fn week_numbers() {
        // 2023-01-01 was a Sunday: it opens %U week 1 but sits in %W week 0.
        let t = unix(2023, 1, 1, 12, 0, 0);
        assert_eq!(fmt(t, b'w'), "0");
        assert_eq!(fmt(t, b'U'), "01");
        assert_eq!(fmt(t, b'W'), "00");
        // 2024-01-01 was a Monday: %U week 0, %W week 1.
        let t = unix(2024, 1, 1, 12, 0, 0);
        assert_eq!(fmt(t, b'w'), "1");
        assert_eq!(fmt(t, b'U'), "00");
        assert_eq!(fmt(t, b'W'), "01");
        // 1970-01-04 was the first Sunday of 1970: %U ticks to 1 there.
        assert_eq!(fmt(2 * 86_400, b'U'), "00"); // Sat Jan 3
        assert_eq!(fmt(3 * 86_400, b'U'), "01"); // Sun Jan 4
    }

    #[test]
    fn hour_edges() {
        let noon = unix(2000, 6, 10, 12, 0, 0);
        assert_eq!(fmt(noon, b'I'), "12");
        assert_eq!(fmt(noon, b'p'), "PM");
        let almost_midnight = unix(2000, 6, 10, 23, 5, 0);
        assert_eq!(fmt(almost_midnight, b'I'), "11");
        assert_eq!(fmt(almost_midnight, b'p'), "PM");
        let one_am = unix(2000, 6, 10, 1, 0, 0);
        assert_eq!(fmt(one_am, b'I'), "01");
        assert_eq!(fmt(one_am, b'p'), "AM");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn offset_and_zone_from_tm_fields() {
        let mut t = utc_tm(0);
        assert_eq!(strftime_directive(&t, b'z'), "+0000");
        assert_eq!(strftime_directive(&t, b'Z'), ""); // no zone info attached
        t.tm_gmtoff = 3600;
        assert_eq!(strftime_directive(&t, b'z'), "+0100");
        t.tm_gmtoff = -16_200; // -04:30
        assert_eq!(strftime_directive(&t, b'z'), "-0430");
        t.tm_zone = c"UTC".as_ptr();
        assert_eq!(strftime_directive(&t, b'Z'), "UTC");
        t.tm_zone = c"CET".as_ptr();
        assert_eq!(strftime_directive(&t, b'Z'), "CET");
    }

    /// Cross-check every specifier Luau accepts against the platform's C
    /// `strftime` — the exact code path native `os.date` used before — over
    /// UTC *and* local broken-down times for a spread of timestamps (leap
    /// days, year boundaries, Y2038, pre-epoch). Both implementations read
    /// only the `tm` fields, and the process is in the C locale, so every row
    /// is deterministic. Unix only: the MSVC `tm` layout differs and its
    /// `%z`/`%Z` are localized-name quirks this port deliberately drops.
    #[cfg(unix)]
    mod c_oracle {
        use super::*;
        use crate::functions::localtime_r::{localtime_r, time_t};
        use core::ffi::c_char;

        extern "C" {
            fn strftime(s: *mut c_char, max: usize, format: *const c_char, tm: *const tm) -> usize;
            fn gmtime_r(timep: *const time_t, result: *mut tm) -> *mut tm;
        }

        // `%z` is oracle-checked on glibc only: glibc renders the tm-carried
        // `tm_gmtoff` (the semantics this port unifies on), while BSD/macOS
        // strftime ignores it and recomputes from the process-global timezone
        // — the very platform divergence native `os.date("!%z")` used to
        // exhibit. Linux CI pins the glibc agreement; the unit tests above pin
        // `%z` on every target.
        #[cfg(target_os = "linux")]
        const SPECIFIERS: &[u8] = b"aAbBcdHIjmMpSUwWxXyYzZ%";
        #[cfg(not(target_os = "linux"))]
        const SPECIFIERS: &[u8] = b"aAbBcdHIjmMpSUwWxXyYZ%";

        fn timestamps() -> alloc::vec::Vec<i64> {
            let mut ts = alloc::vec![
                0,
                1,
                59,
                3599,
                86_399,
                86_400,
                -1,
                -86_400,
                1_234_567_890,
                2_147_483_647, // i32 max (2038-01-19)
                2_147_483_648,
                32_535_215_999, // 3000-12-31, the conformance suite's far edge
                unix(2000, 2, 29, 12, 30, 45),
                unix(2016, 2, 29, 0, 0, 0),
                unix(2024, 2, 29, 23, 59, 59),
                unix(2020, 12, 31, 23, 59, 59),
                unix(2021, 1, 1, 0, 0, 0),
                unix(1999, 12, 31, 23, 59, 59),
            ];
            // Jan 1 of consecutive years covers every weekday for %U/%W.
            for y in 2014..=2026 {
                ts.push(unix(y, 1, 1, 6, 7, 8));
                ts.push(unix(y, 12, 31, 18, 9, 10));
            }
            ts
        }

        fn libc_render(t: &tm, conv: u8) -> String {
            let format: [c_char; 3] = [b'%' as c_char, conv as c_char, 0];
            let mut buf = [0u8; 256];
            let n = unsafe {
                strftime(
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len(),
                    format.as_ptr(),
                    t,
                )
            };
            String::from_utf8(buf[..n].to_vec()).unwrap()
        }

        #[test]
        fn utc_matches_libc_strftime() {
            for secs in timestamps() {
                let mut t: tm = unsafe { core::mem::zeroed() };
                assert!(!unsafe { gmtime_r(&secs, &mut t) }.is_null());
                for &conv in SPECIFIERS {
                    assert_eq!(
                        strftime_directive(&t, conv),
                        libc_render(&t, conv),
                        "%{} of {secs} (utc)",
                        conv as char
                    );
                }
            }
        }

        #[test]
        fn local_matches_libc_strftime() {
            for secs in timestamps() {
                if secs < 0 {
                    continue; // os.date disallows pre-epoch local time
                }
                let mut t: tm = unsafe { core::mem::zeroed() };
                assert!(!unsafe { localtime_r(&secs, &mut t) }.is_null());
                for &conv in SPECIFIERS {
                    assert_eq!(
                        strftime_directive(&t, conv),
                        libc_render(&t, conv),
                        "%{} of {secs} (local)",
                        conv as char
                    );
                }
            }
        }
    }
}
