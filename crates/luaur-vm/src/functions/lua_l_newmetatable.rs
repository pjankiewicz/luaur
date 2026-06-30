use crate::functions::lua_getfield::lua_getfield;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::functions::lua_setfield::lua_setfield;
use crate::functions::lua_type::lua_type;
use crate::macros::lua_newtable::lua_newtable;
use crate::macros::lua_pop::lua_pop;
use crate::macros::lua_registryindex::LUA_REGISTRYINDEX;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

#[export_name = "luaur_lua_l_newmetatable"]
pub unsafe fn lua_l_newmetatable(L: *mut lua_State, tname: *const c_char) -> c_int {
    lua_getfield(L, LUA_REGISTRYINDEX, tname);

    if lua_type(L, -1) != (crate::enums::lua_type::lua_Type::LUA_TNIL as i32) {
        return 0;
    }

    lua_pop(L, 1);
    lua_newtable(L);

    lua_pushvalue(L, -1);

    lua_setfield(L, LUA_REGISTRYINDEX, tname);

    1
}
