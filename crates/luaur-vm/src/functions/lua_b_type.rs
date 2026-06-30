use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_pushstring::lua_pushstring;
use crate::functions::lua_type::lua_type;
use crate::functions::lua_typename::lua_typename;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_type"]
pub unsafe fn lua_b_type(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);
    // resulting name doesn't differentiate between userdata types
    let t = lua_type(L, 1);
    let name = lua_typename(L, t);
    lua_pushstring(L, name);
    1
}
