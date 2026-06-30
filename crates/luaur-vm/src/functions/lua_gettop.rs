use crate::macros::cast_int::cast_int;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_gettop"]
pub unsafe fn lua_gettop(L: *mut lua_State) -> core::ffi::c_int {
    cast_int!((*L).top.offset_from((*L).base))
}
