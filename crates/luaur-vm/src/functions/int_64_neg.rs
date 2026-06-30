use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_neg"]
pub unsafe fn int64_neg(l: *mut LuaState) -> core::ffi::c_int {
    let x = lua_l_checkinteger_64(l, 1);

    lua_pushinteger_64(l, (x as u64).wrapping_neg() as i64);

    1
}
