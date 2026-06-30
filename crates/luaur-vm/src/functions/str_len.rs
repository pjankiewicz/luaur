use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_str_len"]
pub unsafe fn str_len(l: *mut lua_State) -> core::ffi::c_int {
    let mut len: usize = 0;
    lua_l_checklstring(l, 1, &mut len);
    lua_pushinteger(l, len as core::ffi::c_int);
    1
}
