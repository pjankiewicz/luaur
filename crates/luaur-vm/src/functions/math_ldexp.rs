use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_ldexp"]
pub unsafe fn math_ldexp(L: *mut lua_State) -> i32 {
    let x = lua_l_checknumber(L, 1);
    let exp = lua_l_checkinteger(L, 2);
    // ldexp(x, exp) is x * 2^exp
    lua_pushnumber(L, x * (2.0f64).powi(exp as i32));
    1
}
