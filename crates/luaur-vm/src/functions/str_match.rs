use crate::functions::str_find_aux::str_find_aux;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_str_match"]
pub unsafe fn str_match(l: *mut lua_State) -> core::ffi::c_int {
    str_find_aux(l, 0)
}
