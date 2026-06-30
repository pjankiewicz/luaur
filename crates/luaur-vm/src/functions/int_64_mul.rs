use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_mul"]
pub unsafe fn int64_mul(l: *mut lua_State) -> core::ffi::c_int {
    let x = lua_l_checkinteger_64(l, 1);
    let y = lua_l_checkinteger_64(l, 2);

    lua_pushinteger_64(l, ((x as u64).wrapping_mul(y as u64)) as i64);

    1
}
