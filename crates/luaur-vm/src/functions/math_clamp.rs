use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_clamp"]
pub unsafe fn math_clamp(l: *mut lua_State) -> i32 {
    let v = lua_l_checknumber(l, 1);
    let min = lua_l_checknumber(l, 2);
    let max = lua_l_checknumber(l, 3);

    luaL_argcheck!(l, min <= max, 3, "max must be greater than or equal to min");

    let r = if v < min { min } else { v };
    let r = if r > max { max } else { r };

    lua_pushnumber(l, r);
    1
}
