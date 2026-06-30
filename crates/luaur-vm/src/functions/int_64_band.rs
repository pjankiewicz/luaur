use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_band"]
pub unsafe fn int64_band(l: *mut lua_State) -> core::ffi::c_int {
    let mut tres: u64 = u64::MAX;
    let n = lua_gettop(l);

    for i in 1..=n {
        let x = lua_l_checkinteger_64(l, i) as u64;
        tres &= x;
    }

    lua_pushinteger_64(l, tres as i64);
    1
}
