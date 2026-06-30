use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::u_posrelat::u_posrelat;
use crate::functions::utf_8_decode::utf_8_decode;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_utflen"]
pub unsafe fn utflen(L: *mut lua_State) -> c_int {
    let mut n: c_int = 0;
    let mut len: usize = 0;
    let s = lua_l_checklstring(L, 1, &mut len);

    let posi = u_posrelat(lua_l_optinteger(L, 2, 1), len);
    let mut posj = u_posrelat(lua_l_optinteger(L, 3, -1), len);

    luaL_argcheck!(
        L,
        1 <= posi && posi <= len as c_int + 1,
        2,
        "initial position out of string"
    );
    posj -= 1;
    luaL_argcheck!(L, posj < len as c_int, 3, "final position out of string");

    let mut posi = posi - 1;

    while posi <= posj {
        let s1 = utf_8_decode(s.offset(posi as isize), core::ptr::null_mut());
        if s1.is_null() {
            lua_pushnil(L);
            lua_pushinteger(L, (posi + 1) as c_int);
            return 2;
        }
        posi = (s1 as isize - s as isize) as c_int;
        n += 1;
    }

    lua_pushinteger(L, n);
    1
}
