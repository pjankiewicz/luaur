use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_add"]
pub unsafe fn int64_add(l: *mut LuaState) -> core::ffi::c_int {
    let x = lua_l_checkinteger_64(l, 1);
    let y = lua_l_checkinteger_64(l, 2);

    let result = (x as u64).wrapping_add(y as u64) as i64;

    lua_pushinteger_64(l, result);

    1
}
