use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_newthread::lua_newthread;
use crate::functions::lua_xpush::lua_xpush;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_cocreate"]
pub unsafe fn cocreate(l: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(l, 1, lua_Type::LUA_TFUNCTION as core::ffi::c_int);

    let nl = lua_newthread(l);
    lua_xpush(l, nl, 1);

    1
}
