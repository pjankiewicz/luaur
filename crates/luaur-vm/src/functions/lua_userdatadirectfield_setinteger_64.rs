use crate::macros::setlvalue::setlvalue;
use crate::type_aliases::t_value::TValue;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[export_name = "luaur_lua_userdatadirectfield_setinteger64"]
pub unsafe fn lua_userdatadirectfield_setinteger64(result: *mut core::ffi::c_void, n: i64) {
    LUAU_ASSERT!(luaur_common::FFlag::LuauDirectFieldGet.get());
    setlvalue!(result as *mut TValue, n);
}

#[export_name = "luaur_lua_userdatadirectfield_setinteger_64"]
pub unsafe fn lua_userdatadirectfield_setinteger_64(result: *mut core::ffi::c_void, n: i64) {
    lua_userdatadirectfield_setinteger64(result, n);
}
