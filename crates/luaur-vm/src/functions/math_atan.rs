use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_atan"]
pub unsafe fn math_atan(L: *mut lua_State) -> i32 {
    let n = lua_l_checknumber(L, 1);
    lua_pushnumber(L, n.atan());
    1
}
