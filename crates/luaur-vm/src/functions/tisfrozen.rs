use crate::enums::lua_type::lua_Type;
use crate::functions::lua_getreadonly::lua_getreadonly;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_tisfrozen"]
pub unsafe fn tisfrozen(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);

    lua_pushboolean(L, lua_getreadonly(L, 1));

    1
}
