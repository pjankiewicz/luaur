use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_max"]
pub unsafe fn int64_max(l: *mut LuaState) -> core::ffi::c_int {
    let mut tmax: i64 = lua_l_checkinteger_64(l, 1);
    let n = lua_gettop(l);

    for i in 2..=n {
        let x = lua_l_checkinteger_64(l, i);
        if x > tmax {
            tmax = x;
        }
    }

    lua_pushinteger_64(l, tmax);

    1
}
