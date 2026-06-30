use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::luaL_checkstring;
use crate::macros::lua_pushcfunction::LUA_PUSHCFUNCTION;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_iter_codes"]
pub unsafe fn iter_codes(L: *mut lua_State) -> core::ffi::c_int {
    luaL_checkstring!(L, 1);
    LUA_PUSHCFUNCTION(
        L,
        Some(crate::functions::iter_aux::iter_aux),
        core::ptr::null(),
    );
    lua_pushvalue(L, 1);
    lua_pushinteger(L, 0);
    3
}
