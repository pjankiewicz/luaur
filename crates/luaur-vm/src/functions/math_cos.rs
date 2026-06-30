use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_cos"]
pub unsafe fn math_cos(l: *mut lua_State) -> i32 {
    lua_pushnumber(l, f64::cos(lua_l_checknumber(l, 1)));
    1
}
