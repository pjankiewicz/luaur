use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_lerp"]
pub unsafe fn math_lerp(l: *mut lua_State) -> i32 {
    let a = lua_l_checknumber(l, 1);
    let b = lua_l_checknumber(l, 2);
    let t = lua_l_checknumber(l, 3);

    let r = if t == 1.0 { b } else { a + (b - a) * t };

    lua_pushnumber(l, r);
    1
}
