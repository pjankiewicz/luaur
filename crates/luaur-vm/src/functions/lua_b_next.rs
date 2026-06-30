use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_next::lua_next;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_settop::lua_settop;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_lua_b_next"]
pub unsafe fn lua_b_next(L: *mut lua_State) -> c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as c_int);
    lua_settop(L, 2);

    if lua_next(L, 1) != 0 {
        2
    } else {
        lua_pushnil(L);
        1
    }
}
