use crate::enums::lua_type::lua_Type;
use crate::functions::lua_call::lua_call;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::functions::lua_rawgeti::lua_rawgeti;
use crate::macros::lua_isnil::lua_isnil;
use crate::macros::lua_pop::lua_pop;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_foreachi"]
pub unsafe fn foreachi(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    lua_l_checktype(L, 2, lua_Type::LUA_TFUNCTION as core::ffi::c_int);

    let mut i: core::ffi::c_int = 1;
    let n = lua_objlen(L, 1);

    while i <= n {
        lua_pushvalue(L, 2); // function
        lua_pushinteger(L, i); // 1st argument
        lua_rawgeti(L, 1, i); // 2nd argument
        lua_call(L, 2, 1);

        if !lua_isnil!(L, -1) {
            return 1;
        }
        lua_pop(L, 1); // remove nil result

        i += 1;
    }

    0
}
