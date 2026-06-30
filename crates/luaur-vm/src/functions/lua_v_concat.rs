//! Node: `cxx:Function:Luau.VM:VM/src/lvmutils.cpp:399:luaV_concat`
//! Source: `VM/src/lvmutils.cpp:399-461` (hand-ported)

use crate::functions::call_bin_tm::call_bin_tm;
use crate::functions::lua_g_concaterror::lua_g_concaterror;
use crate::functions::lua_s_buffinish::luaS_buffinish;
use crate::functions::lua_s_bufstart::luaS_bufstart;
use crate::functions::lua_s_newlstr::luaS_newlstr;
use crate::macros::lua_buffersize::LUA_BUFFERSIZE;
use crate::macros::lua_g_runerror::lua_g_runerror;
use crate::macros::maxssize::MAXSSIZE;
use crate::macros::setsvalue::setsvalue;
use crate::macros::svalue::svalue;
use crate::macros::tostring::tostring;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttisstring::ttisstring;
use crate::records::t_string::TString;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::tms::TMS;
use core::ffi::c_char;

#[allow(non_snake_case)]
pub unsafe fn lua_v_concat(L: *mut lua_State, mut total: i32, mut last: i32) {
    loop {
        let top: StkId = (*L).base.add((last + 1) as usize);
        let mut n = 2; // number of elements handled in this pass (at least 2)
        if !(ttisstring!(top.sub(2)) || ttisnumber!(top.sub(2))) || !tostring!(L, top.sub(1)) {
            if call_bin_tm(L, top.sub(2), top.sub(1), top.sub(2), TMS::TM_CONCAT) == 0 {
                lua_g_concaterror(L, top.sub(2), top.sub(1));
            }
        } else if (*tsvalue!(top.sub(1))).len == 0 {
            // second op is empty? result is first op (as string)
            let _ = tostring!(L, top.sub(2));
        } else {
            // at least two string values; get as many as possible
            let mut tl = (*tsvalue!(top.sub(1))).len as usize;
            // collect total length
            n = 1;
            while n < total && tostring!(L, top.sub((n + 1) as usize)) {
                let l = (*tsvalue!(top.sub((n + 1) as usize))).len as usize;
                if l > MAXSSIZE as usize - tl {
                    lua_g_runerror!(L, "string length overflow");
                }
                tl += l;
                n += 1;
            }

            let mut buf = [0 as c_char; LUA_BUFFERSIZE as usize];
            let mut ts: *mut TString = core::ptr::null_mut();

            let buffer: *mut c_char = if tl < LUA_BUFFERSIZE as usize {
                buf.as_mut_ptr()
            } else {
                ts = luaS_bufstart(L, tl);
                (*ts).data.as_mut_ptr()
            };

            // concat all strings
            tl = 0;
            let mut i = n;
            while i > 0 {
                let l = (*tsvalue!(top.sub(i as usize))).len as usize;
                core::ptr::copy_nonoverlapping(svalue!(top.sub(i as usize)), buffer.add(tl), l);
                tl += l;
                i -= 1;
            }

            if tl < LUA_BUFFERSIZE as usize {
                setsvalue!(L, top.sub(n as usize), luaS_newlstr(L, buffer, tl));
            } else {
                setsvalue!(L, top.sub(n as usize), luaS_buffinish(L, ts));
            }
        }
        total -= n - 1; // got `n` strings to create 1 new
        last -= n - 1;
        if total <= 1 {
            break; // repeat until only 1 result left
        }
    }
}

#[export_name = "luaur_luaV_concat"]
pub unsafe extern "C" fn lua_v_concat_export(L: *mut lua_State, total: i32, last: i32) {
    lua_v_concat(L, total, last);
}

#[allow(non_snake_case, unused_imports)]
pub use lua_v_concat as luaV_concat;
