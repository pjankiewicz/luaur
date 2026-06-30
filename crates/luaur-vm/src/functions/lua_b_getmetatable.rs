use crate::functions::lua_getmetatable::lua_getmetatable;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_getmetafield::lua_l_getmetafield;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_getmetatable"]
pub unsafe fn lua_b_getmetatable(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);

    if lua_getmetatable(L, 1) == 0 {
        lua_pushnil(L);
        return 1; // no metatable
    }

    lua_l_getmetafield(L, 1, c"__metatable".as_ptr());
    1 // returns either __metatable field (if present) or metatable
}
