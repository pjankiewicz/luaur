use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_rawget::lua_rawget;
use crate::functions::lua_settop::lua_settop;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_b_rawget"]
pub unsafe fn lua_b_rawget(l: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(l, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    lua_l_checkany(l, 2);
    lua_settop(l, 2);
    lua_rawget(l, 1);
    1
}
