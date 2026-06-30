use core::ffi::c_int;

use crate::enums::lua_type::lua_Type;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_type::lua_type;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_rawlen"]
pub unsafe fn lua_b_rawlen(L: *mut lua_State) -> c_int {
    let tt = lua_type(L, 1);

    luaL_argcheck!(
        L,
        tt == lua_Type::LUA_TTABLE as c_int || tt == lua_Type::LUA_TSTRING as c_int,
        1,
        "table or string expected"
    );

    let len = lua_objlen(L, 1);
    lua_pushinteger(L, len);

    1
}
