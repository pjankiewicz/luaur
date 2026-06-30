use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_lshift"]
pub unsafe fn int64_lshift(L: *mut lua_State) -> core::ffi::c_int {
    let n = lua_l_checkinteger_64(L, 1) as u64;
    let i = lua_l_checkinteger_64(L, 2);

    if i >= -63 && i <= 63 {
        lua_pushinteger_64(L, if i < 0 { n >> (-i) } else { n << i } as i64);
    } else {
        lua_pushinteger_64(L, 0);
    }

    1
}
