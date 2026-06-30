use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_math_isfinite"]
pub unsafe fn math_isfinite(L: *mut lua_State) -> i32 {
    let x = lua_l_checknumber(L, 1);
    lua_pushboolean(L, x.is_finite() as i32);
    1
}
