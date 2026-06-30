use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_settop::lua_settop;
use crate::luaL_checkstring;
use crate::macros::lua_pushcclosure::lua_pushcclosure;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_gmatch"]
pub unsafe fn gmatch(L: *mut lua_State) -> core::ffi::c_int {
    luaL_checkstring!(L, 1);
    luaL_checkstring!(L, 2);
    lua_settop(L, 2);
    lua_pushinteger(L, 0);
    lua_pushcclosure(
        L,
        Some(crate::functions::gmatch_aux::gmatch_aux),
        core::ptr::null(),
        3,
    );
    1
}
