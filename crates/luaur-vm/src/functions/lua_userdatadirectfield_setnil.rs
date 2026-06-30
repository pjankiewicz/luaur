use crate::macros::setnilvalue::setnilvalue;
use crate::type_aliases::t_value::TValue;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
#[export_name = "luaur_lua_userdatadirectfield_setnil"]
pub unsafe fn lua_userdatadirectfield_setnil(result: *mut core::ffi::c_void) {
    LUAU_ASSERT!(luaur_common::FFlag::LuauDirectFieldGet.get());
    setnilvalue!(result as *mut TValue);
}
