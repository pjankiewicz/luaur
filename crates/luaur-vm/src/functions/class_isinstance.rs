use crate::enums::lua_type::lua_Type;
use crate::functions::lua_a_toobject::luaA_toobject;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::macros::classvalue::classvalue;
use crate::macros::objectvalue::objectvalue;
use crate::macros::ttisobject::ttisobject;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_class_isinstance"]
pub unsafe fn class_isinstance(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);
    lua_l_checktype(L, 2, lua_Type::LUA_TCLASS as core::ffi::c_int);

    let inst: *const TValue = luaA_toobject(L, 1);
    let obj: *const TValue = luaA_toobject(L, 2);

    // classvalue! returns &mut ManuallyDrop<LuauClass>. We cast to raw pointer for comparison.
    let lclass = classvalue!(obj) as *mut _ as *mut crate::records::luau_class::LuauClass;

    // ttisobject! returns a bool.
    // objectvalue! returns &mut ManuallyDrop<LuauObject>.
    let is_instance = ttisobject!(inst) && {
        let obj_ptr = objectvalue!(inst) as *mut _ as *mut crate::records::luau_object::LuauObject;
        (*obj_ptr).lclass == lclass
    };

    lua_pushboolean(L, is_instance as core::ffi::c_int);
    1
}
