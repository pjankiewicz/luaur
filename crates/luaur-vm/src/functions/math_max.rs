use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_max"]
pub unsafe fn math_max(l: *mut lua_State) -> i32 {
    let n = lua_gettop(l); // number of arguments
    let mut dmax = lua_l_checknumber(l, 1);
    let mut i = 2;
    while i <= n {
        let d = lua_l_checknumber(l, i);
        if d > dmax {
            dmax = d;
        }
        i += 1;
    }
    lua_pushnumber(l, dmax);
    1
}
