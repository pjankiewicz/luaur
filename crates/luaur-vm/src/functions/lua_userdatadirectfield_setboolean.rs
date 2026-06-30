use crate::macros::setbvalue::setbvalue;
use crate::type_aliases::t_value::TValue;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[export_name = "luaur_lua_userdatadirectfield_setboolean"]
#[allow(non_snake_case)]
pub unsafe fn lua_userdatadirectfield_setboolean(
    result: *mut core::ffi::c_void,
    b: core::ffi::c_int,
) {
    LUAU_ASSERT!(luaur_common::FFlag::LuauDirectFieldGet.get());
    setbvalue!(result as *mut TValue, b);
}
