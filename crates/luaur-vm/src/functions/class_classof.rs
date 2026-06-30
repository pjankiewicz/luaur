use crate::functions::lua_a_toobject::luaA_toobject;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::macros::lua_isobject::lua_isobject;
use crate::macros::objectvalue::objectvalue;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_class_classof"]
pub unsafe fn class_classof(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);

    if !lua_isobject!(L, 1) {
        lua_pushnil(L);
        return 1;
    }

    let inst: *const crate::type_aliases::t_value::TValue = luaA_toobject(L, 1);
    let ci = objectvalue!(inst);
    crate::functions::lua_a_pushclass::luaA_pushclass(L, (*ci).lclass);
    1
}
