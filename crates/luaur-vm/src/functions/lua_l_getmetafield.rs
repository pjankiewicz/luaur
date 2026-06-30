use crate::functions::lua_getmetatable::lua_getmetatable;
use crate::functions::lua_pushstring::lua_pushstring;
use crate::functions::lua_rawget::lua_rawget;
use crate::functions::lua_remove::lua_remove;
use crate::macros::lua_pop::lua_pop;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

#[export_name = "luaur_lua_l_getmetafield"]
pub unsafe fn lua_l_getmetafield(L: *mut lua_State, obj: c_int, event: *const c_char) -> c_int {
    if lua_getmetatable(L, obj) == 0 {
        return 0; // no metatable
    }

    lua_pushstring(L, event);
    lua_rawget(L, -2);

    let is_nil = crate::functions::lua_type::lua_type(L, -1)
        == (crate::enums::lua_type::lua_Type::LUA_TNIL as i32);

    if is_nil {
        lua_pop(L, 2); // remove metatable and metafield
        return 0;
    } else {
        lua_remove(L, -2); // remove only metatable
        return 1;
    }
}
