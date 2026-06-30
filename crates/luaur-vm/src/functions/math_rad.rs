use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::macros::radians_per_degree::RADIANS_PER_DEGREE;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_rad"]
pub unsafe fn math_rad(L: *mut lua_State) -> i32 {
    lua_pushnumber(L, lua_l_checknumber(L, 1) * RADIANS_PER_DEGREE);
    1
}
