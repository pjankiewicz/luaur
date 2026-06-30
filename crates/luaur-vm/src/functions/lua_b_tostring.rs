use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_tolstring::lua_l_tolstring;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_tostring"]
pub unsafe fn lua_b_tostring(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);
    lua_l_tolstring(L, 1, core::ptr::null_mut());
    1
}
