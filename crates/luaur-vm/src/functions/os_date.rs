//! Node: `cxx:Function:Luau.VM:VM/src/loslib.cpp:112:os_date`
//!
//! `os.date` — format a timestamp. An optional `!` prefix selects UTC; the format
//! `*t` builds a table of broken-down fields; otherwise each `%` conversion spec
//! is rendered through the pure-Rust directive renderer (the C++ original
//! forwards to `strftime`, which `wasm32-unknown-unknown` cannot bind — no libc
//! — so the rendering is implemented natively for every target; see
//! `strftime_directive` for the C-locale / timezone policy). The broken-down
//! time still comes from `time`/`gmtime_r`/`localtime_r`, which the wasm build
//! shims in `luaur-common::wasm_libc` (fixed clock, local == UTC).

use crate::functions::localtime_r::{localtime_r, time_t, tm};
use crate::functions::lua_createtable::lua_createtable;
use crate::functions::lua_l_addlstring::lua_l_addlstring;
use crate::functions::lua_l_buffinit::lua_l_buffinit;
use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_l_pushresult::lua_l_pushresult;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::setboolfield::setboolfield;
use crate::functions::setfield::setfield;
use crate::functions::strftime_directive::strftime_directive;
use crate::macros::lua_isnoneornil::lua_isnoneornil;
use crate::macros::lua_l_addchar::luaL_addchar;
use crate::macros::lua_l_argerror::luaL_argerror;
use crate::macros::lua_l_optstring::luaL_optstring;
use crate::macros::lua_strftimeoptions::LUA_STRFTIMEOPTIONS;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

extern "C" {
    fn time(t: *mut time_t) -> time_t;
}

/// `gmtime_r` wrapper. The graph's `gmtime_r` dep unconditionally calls Windows
/// `gmtime_s`, so we declare the platform-correct symbol directly here.
unsafe fn os_gmtime_r(timep: *const time_t, result: *mut tm) -> *mut tm {
    #[cfg(target_os = "windows")]
    {
        extern "C" {
            // `gmtime_s` is inline in MSVC's <time.h>; link the real UCRT export
            // `_gmtime64_s` (__time64_t = i64) instead.
            fn _gmtime64_s(result: *mut tm, timep: *const time_t) -> c_int;
        }
        if _gmtime64_s(result, timep) == 0 {
            result
        } else {
            core::ptr::null_mut()
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        extern "C" {
            fn gmtime_r(timep: *const time_t, result: *mut tm) -> *mut tm;
        }
        gmtime_r(timep, result)
    }
}

pub unsafe fn os_date(L: *mut lua_State) -> c_int {
    let mut s: *const c_char = luaL_optstring!(L, 1, b"%c\0".as_ptr() as *const c_char);
    let t: time_t = if lua_isnoneornil!(L, 2) {
        time(core::ptr::null_mut())
    } else {
        lua_l_checknumber(L, 2) as time_t
    };

    let mut tmv: tm = core::mem::zeroed();
    let stm: *mut tm;
    if *s == b'!' as c_char {
        // UTC?
        stm = os_gmtime_r(&t, &mut tmv);
        s = s.add(1); // skip '!'
    } else {
        // localtime fails for dates before the epoch on some platforms, so disallow that
        stm = if t < 0 {
            core::ptr::null_mut()
        } else {
            localtime_r(&t, &mut tmv)
        };
    }

    if stm.is_null() {
        // invalid date?
        lua_pushnil(L);
    } else if c_str_eq(s, b"*t\0") {
        lua_createtable(L, 0, 9); // 9 = number of fields
        setfield(L, "sec", (*stm).tm_sec);
        setfield(L, "min", (*stm).tm_min);
        setfield(L, "hour", (*stm).tm_hour);
        setfield(L, "day", (*stm).tm_mday);
        setfield(L, "month", (*stm).tm_mon + 1);
        setfield(L, "year", (*stm).tm_year + 1900);
        setfield(L, "wday", (*stm).tm_wday + 1);
        setfield(L, "yday", (*stm).tm_yday + 1);
        setboolfield(L, "isdst", (*stm).tm_isdst);
    } else {
        let mut b: LuaLStrbuf = LuaLStrbuf {
            p: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            L: core::ptr::null_mut(),
            storage: core::ptr::null_mut(),
            buffer: [0; 512],
        };
        lua_l_buffinit(L, &mut b);

        while *s != 0 {
            let c = *s;
            let next = *s.add(1);
            if c != b'%' as c_char || next == 0 {
                // no conversion specifier?
                luaL_addchar!(&mut b, c);
            } else if !strftime_option_contains(next) {
                luaL_argerror!(L, 1, "invalid conversion specifier");
            } else {
                s = s.add(1);
                let rendered = strftime_directive(&*stm, *s as u8);
                lua_l_addlstring(&mut b, rendered.as_ptr() as *const c_char, rendered.len());
            }
            s = s.add(1);
        }
        lua_l_pushresult(&mut b);
    }
    1
}

/// `strcmp(s, lit) == 0` where `lit` is a NUL-terminated byte literal.
unsafe fn c_str_eq(s: *const c_char, lit: &[u8]) -> bool {
    let mut i = 0;
    loop {
        let a = *s.add(i) as u8;
        let b = lit[i];
        if a != b {
            return false;
        }
        if b == 0 {
            return true;
        }
        i += 1;
    }
}

/// Membership of `c` in `LUA_STRFTIMEOPTIONS` (the C++ `strchr(...) != 0` test).
fn strftime_option_contains(c: c_char) -> bool {
    LUA_STRFTIMEOPTIONS
        .as_bytes()
        .iter()
        .any(|&o| o as c_char == c)
}
