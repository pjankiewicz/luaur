//! `luaA_pushclass` — push a `LuauClass*` value onto the Lua stack.
//! C++ source: `VM/src/lapi.cpp:133`

use crate::enums::lua_type::lua_Type;
use crate::records::gc_object::GCObject;
use crate::records::lua_state::lua_State;
use crate::type_aliases::luau_class::LuauClass;

#[export_name = "luaur_luaA_pushclass"]
#[allow(non_snake_case)]
pub unsafe fn luaA_pushclass(l: *mut lua_State, lco: *mut LuauClass) {
    crate::api_check!(l, !lco.is_null());

    let i_o = (*l).top;
    (*i_o).value.gc = lco as *mut GCObject;
    (*i_o).set_tt(lua_Type::LUA_TCLASS as core::ffi::c_int);

    crate::api_incr_top!(l);
}
