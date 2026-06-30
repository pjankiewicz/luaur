use crate::records::call_info::CallInfo;
use crate::records::lua_state::lua_State;
use crate::type_aliases::lua_state::lua_State as LuaState;

#[export_name = "luaur_lua_stackdepth"]
pub unsafe fn lua_stackdepth(L: *mut lua_State) -> core::ffi::c_int {
    (*L).ci.offset_from((*L).base_ci) as core::ffi::c_int
}
