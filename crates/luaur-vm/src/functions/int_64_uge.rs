use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_uge"]
pub unsafe fn int64_uge(l: *mut LuaState) -> core::ffi::c_int {
    let a = lua_l_checkinteger_64(l, 1) as u64;
    let b = lua_l_checkinteger_64(l, 2) as u64;

    lua_pushboolean(l, (a >= b) as core::ffi::c_int);

    1
}
