use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_l_optnumber::lua_l_optnumber;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_os_difftime"]
pub unsafe fn os_difftime(L: *mut lua_State) -> i32 {
    let t1 = lua_l_checknumber(L, 1);
    let t2 = lua_l_optnumber(L, 2, 0.0);

    // difftime in C returns the difference in seconds (t1 - t2) as a double.
    // Since we are targeting wasm32-unknown-unknown and portable environments,
    // and the input numbers are already doubles from the Lua stack, we can
    // perform the subtraction directly.
    let result = t1 - t2;

    lua_pushnumber(L, result);
    1
}
