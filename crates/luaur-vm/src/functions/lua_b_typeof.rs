use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_typename::lua_l_typename;
use crate::functions::lua_pushstring::lua_pushstring;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_typeof"]
pub unsafe fn lua_b_typeof(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);
    let name = lua_l_typename(L, 1);
    lua_pushstring(L, name);
    1
}
