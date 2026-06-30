use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_fmod"]
pub unsafe fn math_fmod(L: *mut lua_State) -> i32 {
    lua_pushnumber(
        L,
        f64::from(fmod(lua_l_checknumber(L, 1), lua_l_checknumber(L, 2))),
    );
    1
}

#[inline]
fn fmod(a: f64, b: f64) -> f64 {
    a % b
}
