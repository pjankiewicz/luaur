use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checkinteger::luaL_checkinteger;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_rawgeti::lua_rawgeti;
use crate::macros::lua_isnil::lua_isnil;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_inext"]
pub unsafe fn lua_b_inext(l: *mut lua_State) -> i32 {
    let mut i = luaL_checkinteger(l, 2);
    lua_l_checktype(l, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    i += 1; // next value
    lua_pushinteger(l, i);
    lua_rawgeti(l, 1, i);

    if lua_isnil!(l, -1) {
        0
    } else {
        2
    }
}
