use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::macros::lua_l_opt::luaL_opt;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_lua_l_optinteger_64"]
pub unsafe fn lua_l_optinteger_64(L: *mut lua_State, narg: c_int, def: i64) -> i64 {
    // The macro luaL_opt! expands to:
    // if lua_isnoneornil(L, narg) { def } else { luaL_checkinteger64(L, narg) }
    // We use the already-translated lua_l_checkinteger_64 as the function argument.
    luaL_opt!(L, lua_l_checkinteger_64, narg, def)
}
