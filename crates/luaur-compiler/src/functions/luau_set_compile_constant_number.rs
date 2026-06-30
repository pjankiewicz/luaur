use crate::functions::set_compile_constant_number::set_compile_constant_number;
use crate::type_aliases::lua_compile_constant::lua_CompileConstant;

#[export_name = "luaur_luau_set_compile_constant_number"]
pub unsafe extern "C" fn luau_set_compile_constant_number(
    constant: *mut lua_CompileConstant,
    n: f64,
) {
    set_compile_constant_number(constant as *mut core::ffi::c_void, n);
}
