use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_status"]
pub unsafe fn lua_status(L: *mut lua_State) -> core::ffi::c_int {
    (*L).status as core::ffi::c_int
}
