use crate::functions::buffutfchar::buffutfchar;
use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_addlstring::lua_l_addlstring;
use crate::functions::lua_l_buffinit::lua_l_buffinit;
use crate::functions::lua_l_pushresult::lua_l_pushresult;
use crate::functions::lua_pushlstring::lua_pushlstring;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::type_aliases::lua_state::lua_State;

use core::ffi::{c_char, c_int};

const UTF8BUFFSZ: usize = 8;

#[export_name = "luaur_utfchar"]
pub unsafe fn utfchar(L: *mut lua_State) -> c_int {
    let mut buff = [0 as c_char; UTF8BUFFSZ];
    let mut charstr = core::ptr::null::<c_char>();

    let n = lua_gettop(L); // number of arguments
    if n == 1 {
        // optimize common case of single char
        let l = buffutfchar(L, 1, buff.as_mut_ptr(), &mut charstr as *mut *const c_char);
        lua_pushlstring(L, charstr, l as usize);
    } else {
        let mut b = LuaLStrbuf {
            p: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            L: core::ptr::null_mut(),
            storage: core::ptr::null_mut(),
            buffer: [0; 512],
        };
        lua_l_buffinit(L, &mut b as *mut LuaLStrbuf);
        let mut i = 1;
        while i <= n {
            let l = buffutfchar(L, i, buff.as_mut_ptr(), &mut charstr as *mut *const c_char);
            lua_l_addlstring(&mut b as *mut LuaLStrbuf, charstr, l as usize);
            i += 1;
        }
        lua_l_pushresult(&mut b as *mut LuaLStrbuf);
    }
    1
}
