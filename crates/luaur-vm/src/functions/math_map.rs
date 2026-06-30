use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_map"]
pub unsafe fn math_map(l: *mut lua_State) -> i32 {
    let x = lua_l_checknumber(l, 1);
    let inmin = lua_l_checknumber(l, 2);
    let inmax = lua_l_checknumber(l, 3);
    let outmin = lua_l_checknumber(l, 4);
    let outmax = lua_l_checknumber(l, 5);

    let result = outmin + (x - inmin) * (outmax - outmin) / (inmax - inmin);
    lua_pushnumber(l, result);
    1
}
