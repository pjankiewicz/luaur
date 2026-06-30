use core::ffi::c_int;

use crate::enums::lua_type::lua_Type;
use crate::functions::lua_type::lua_type;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_isstring"]
#[allow(non_snake_case)]
pub unsafe fn lua_isstring(L: *mut lua_State, idx: c_int) -> c_int {
    let t = lua_type(L, idx);
    if t == lua_Type::LUA_TSTRING as c_int || t == lua_Type::LUA_TNUMBER as c_int {
        1
    } else {
        0
    }
}
