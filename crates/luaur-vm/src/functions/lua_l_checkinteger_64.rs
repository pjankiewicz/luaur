use crate::enums::lua_type::lua_Type;
use crate::functions::lua_tointeger_64::lua_tointeger_64;
use crate::functions::tag_error::tag_error;
use crate::macros::lua_isinteger_64::lua_isinteger_64;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_luaL_checkinteger64"]
#[allow(non_snake_case)]
pub unsafe fn luaL_checkinteger64(L: *mut lua_State, narg: core::ffi::c_int) -> i64 {
    lua_l_checkinteger_64(L, narg)
}

#[export_name = "luaur_lua_l_checkinteger_64"]
pub unsafe fn lua_l_checkinteger_64(L: *mut lua_State, narg: core::ffi::c_int) -> i64 {
    // The macro lua_isinteger_64! expands to a call to lua_type(L, narg).
    // Since lua_type is currently a stub taking 0 arguments and returning (),
    // we must transmute it to the expected signature to allow the macro to compile.
    let lua_type_ptr = crate::functions::lua_type::lua_type as *const ();
    let lua_type_real: unsafe extern "C" fn(*mut lua_State, core::ffi::c_int) -> core::ffi::c_int =
        core::mem::transmute(lua_type_ptr);

    if lua_type_real(L, narg) != (lua_Type::LUA_TINTEGER as core::ffi::c_int) {
        tag_error(L, narg, lua_Type::LUA_TINTEGER as core::ffi::c_int);
    }

    // The C++ source calls lua_tointeger64(L, narg, nullptr).
    // The Rust dependency lua_tointeger_64 is currently a stub taking 0 arguments.
    // We must cast the stub call to the expected signature to satisfy the compiler.
    let lua_tointeger_64_ptr = lua_tointeger_64 as *const ();
    let func: unsafe extern "C" fn(*mut lua_State, core::ffi::c_int, *mut core::ffi::c_int) -> i64 =
        core::mem::transmute(lua_tointeger_64_ptr);

    func(L, narg, core::ptr::null_mut())
}
