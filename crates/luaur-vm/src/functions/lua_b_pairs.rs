use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::macros::lua_upvalueindex::lua_upvalueindex;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_pairs"]
pub unsafe fn lua_b_pairs(L: *mut lua_State) -> i32 {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as i32);
    lua_pushvalue(L, lua_upvalueindex(1));
    lua_pushvalue(L, 1);
    lua_pushnil(L);
    3
}
