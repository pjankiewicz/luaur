use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::macros::lua_upvalueindex::lua_upvalueindex;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_ipairs"]
pub unsafe fn lua_b_ipairs(L: *mut lua_State) -> i32 {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as i32);
    lua_pushvalue(L, lua_upvalueindex(1));
    lua_pushvalue(L, 1);
    lua_pushinteger(L, 0);
    3
}
