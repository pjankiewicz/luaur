use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_ge"]
pub unsafe fn int64_ge(l: *mut lua_State) -> core::ffi::c_int {
    let a = lua_l_checkinteger_64(l, 1);
    let b = lua_l_checkinteger_64(l, 2);

    lua_pushboolean(l, (a >= b) as core::ffi::c_int);

    1
}
