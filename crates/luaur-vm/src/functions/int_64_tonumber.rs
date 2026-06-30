use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_tonumber"]
pub unsafe fn int64_tonumber(l: *mut LuaState) -> core::ffi::c_int {
    let x = lua_l_checkinteger_64(l, 1);
    lua_pushnumber(l, x as f64);
    1
}
