use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_atan2"]
#[allow(non_snake_case)]
pub unsafe fn math_atan2(L: *mut lua_State) -> i32 {
    lua_pushnumber(
        L,
        f64::atan2(lua_l_checknumber(L, 1), lua_l_checknumber(L, 2)),
    );
    1
}
