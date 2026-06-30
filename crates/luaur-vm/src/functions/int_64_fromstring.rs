use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_o_str_2_l::lua_o_str_2_l;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::macros::lua_l_checkstring::luaL_checkstring;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_fromstring"]
pub unsafe fn int64_fromstring(L: *mut lua_State) -> core::ffi::c_int {
    let s = luaL_checkstring!(L, 1);
    let base = lua_l_optinteger(L, 2, 10);
    luaL_argcheck!(
        L,
        (2 <= base as i32) && (base as i32 <= 36),
        2,
        "base out of range"
    );

    let mut result: i64 = 0;
    if lua_o_str_2_l(s, &mut result, base as i32) != 0 {
        lua_pushinteger_64(L, result);
    } else {
        lua_pushnil(L);
    }

    1
}
