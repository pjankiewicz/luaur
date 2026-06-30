use crate::enums::lua_type::lua_Type;
use crate::functions::lua_getreadonly::lua_getreadonly;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_getmetafield::lua_l_getmetafield;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::functions::lua_setreadonly::lua_setreadonly;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_tfreeze"]
pub unsafe fn tfreeze(l: *mut lua_State) -> c_int {
    lua_l_checktype(l, 1, lua_Type::LUA_TTABLE as c_int);

    luaL_argcheck!(l, lua_getreadonly(l, 1) == 0, 1, "table is already frozen");

    luaL_argcheck!(
        l,
        lua_l_getmetafield(l, 1, c"__metatable".as_ptr()) == 0,
        1,
        "table has a protected metatable"
    );

    lua_setreadonly(l, 1, 1);

    lua_pushvalue(l, 1);
    1
}
