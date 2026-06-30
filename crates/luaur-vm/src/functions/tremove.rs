use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_rawgeti::lua_rawgeti;
use crate::functions::lua_rawseti::lua_rawseti;
use crate::functions::moveelements::moveelements;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_tremove"]
pub unsafe fn tremove(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    let n = lua_objlen(L, 1);
    let pos = lua_l_optinteger(L, 2, n);

    if !(1 <= pos && pos <= n) {
        return 0;
    }

    lua_rawgeti(L, 1, pos);

    moveelements(L, 1, 1, pos + 1, n, pos);

    lua_pushnil(L);
    lua_rawseti(L, 1, n);

    1
}
