use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_rrotate"]
pub unsafe fn int64_rrotate(l: *mut LuaState) -> core::ffi::c_int {
    let n = lua_l_checkinteger_64(l, 1) as u64;
    let s = (lua_l_checkinteger_64(l, 2) as u64 % 64) as u32;

    let result = if s != 0 {
        (n >> s) | (n << (64 - s))
    } else {
        n
    };

    lua_pushinteger_64(l, result as i64);

    1
}
