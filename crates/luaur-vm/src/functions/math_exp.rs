use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_exp"]
pub unsafe fn math_exp(L: *mut lua_State) -> i32 {
    lua_pushnumber(L, f64::exp(lua_l_checknumber(L, 1)));
    1
}
