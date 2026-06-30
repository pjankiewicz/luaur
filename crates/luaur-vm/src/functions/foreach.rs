use crate::enums::lua_type::lua_Type;
use crate::functions::lua_call::lua_call;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_next::lua_next;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::macros::lua_isnil::lua_isnil;
use crate::macros::lua_pop::lua_pop;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_foreach"]
pub unsafe fn foreach(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    lua_l_checktype(L, 2, lua_Type::LUA_TFUNCTION as core::ffi::c_int);
    lua_pushnil(L); // first key
    while lua_next(L, 1) != 0 {
        lua_pushvalue(L, 2); // function
        lua_pushvalue(L, -3); // key
        lua_pushvalue(L, -3); // value
        lua_call(L, 2, 1);
        if !lua_isnil!(L, -1) {
            return 1;
        }
        lua_pop(L, 2); // remove value and result
    }
    0
}
