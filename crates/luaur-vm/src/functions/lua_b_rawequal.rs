use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::functions::lua_rawequal::lua_rawequal;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_rawequal"]
pub unsafe fn lua_b_rawequal(l: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(l, 1);
    lua_l_checkany(l, 2);

    let result = lua_rawequal(l, 1, 2);
    lua_pushboolean(l, result);
    1
}
