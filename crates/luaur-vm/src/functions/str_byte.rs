use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_checkstack::lua_l_checkstack;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::posrelat::posrelat;
use crate::macros::lua_l_error::luaL_error;
use crate::macros::uchar::uchar;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_str_byte"]
pub unsafe fn str_byte(L: *mut lua_State) -> c_int {
    let mut len: usize = 0;
    let s = lua_l_checklstring(L, 1, &mut len);
    let mut posi = posrelat(lua_l_optinteger(L, 2, 1), len);
    let mut pose = posrelat(lua_l_optinteger(L, 3, posi), len);

    if posi <= 0 {
        posi = 1;
    }
    if (pose as usize) > len {
        pose = len as c_int;
    }

    if posi > pose {
        return 0; // empty interval; return no values
    }

    let n = pose - posi + 1;
    if posi + n <= pose {
        // overflow?
        luaL_error!(L, "string slice too long");
    }

    lua_l_checkstack(L, n, "string slice too long");

    let s_ptr = s.add((posi - 1) as usize);
    for _i in 0..n {
        let val = uchar(*s_ptr.add(_i as usize) as c_int);
        lua_pushinteger(L, val as c_int);
    }

    n
}
