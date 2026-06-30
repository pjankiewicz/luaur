use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushlstring::lua_pushlstring;
use crate::functions::posrelat::posrelat;
use crate::macros::lua_pushliteral::LUA_PUSHLITERAL;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_str_sub"]
pub unsafe fn str_sub(l: *mut lua_State) -> c_int {
    let mut len: usize = 0;
    let s = lua_l_checklstring(l, 1, &mut len);
    let mut start = posrelat(lua_l_checkinteger(l, 2), len);
    let mut end = posrelat(lua_l_optinteger(l, 3, -1), len);

    if start < 1 {
        start = 1;
    }
    if end > len as c_int {
        end = len as c_int;
    }

    if start <= end {
        lua_pushlstring(l, s.add((start - 1) as usize), (end - start + 1) as usize);
    } else {
        LUA_PUSHLITERAL(l as *mut core::ffi::c_void, c"".as_ptr());
    }
    1
}
