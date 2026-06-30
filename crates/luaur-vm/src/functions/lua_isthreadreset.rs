use crate::enums::lua_status::lua_Status;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_isthreadreset"]
pub unsafe fn lua_isthreadreset(L: *mut lua_State) -> core::ffi::c_int {
    ((*L).ci == (*L).base_ci && (*L).base == (*L).top && (*L).status == lua_Status::LUA_OK as u8)
        as core::ffi::c_int
}
