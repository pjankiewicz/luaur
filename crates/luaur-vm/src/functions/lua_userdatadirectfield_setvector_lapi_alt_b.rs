use crate::macros::setvvalue::setvvalue;
use crate::type_aliases::t_value::TValue;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
#[export_name = "luaur_lua_userdatadirectfield_setvector_void_f32_f32_f32"]
pub unsafe fn lua_userdatadirectfield_setvector_void_f32_f32_f32(
    result: *mut core::ffi::c_void,
    x: f32,
    y: f32,
    z: f32,
) {
    LUAU_ASSERT!(luaur_common::FFlag::LuauDirectFieldGet.get());
    setvvalue!(result as *mut TValue, x, y, z, 0.0f32);
}
