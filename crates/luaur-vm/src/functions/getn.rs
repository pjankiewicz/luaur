use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_getn"]
pub unsafe fn getn(l: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(l, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);

    // Note: The dependency card for lua_objlen shows a 0-arg stub `pub fn lua_objlen()`.
    lua_pushinteger(l, lua_objlen(l, 1));

    1
}
