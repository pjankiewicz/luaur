use crate::enums::lua_type::lua_Type;
use crate::functions::lua_toboolean::lua_toboolean;
use crate::functions::tag_error::tag_error;
use crate::macros::lua_isboolean::lua_isboolean;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_l_checkboolean"]
pub unsafe fn lua_l_checkboolean(L: *mut lua_State, narg: core::ffi::c_int) -> core::ffi::c_int {
    // This checks specifically for boolean values, ignoring
    // all other truthy/falsy values. If the desired result
    // is true if value is present then lua_toboolean should
    // directly be used instead.

    // The lua_isboolean! macro depends on lua_type.
    // Since lua_type is currently a 0-arg stub in the dependency card,
    // we must use transmute to call it with the arguments the logic requires.
    let is_bool = {
        let func: unsafe fn(*mut lua_State, core::ffi::c_int) -> core::ffi::c_int =
            core::mem::transmute(crate::functions::lua_type::lua_type as *const core::ffi::c_void);
        func(L, narg) == (lua_Type::LUA_TBOOLEAN as core::ffi::c_int)
    };

    if !is_bool {
        tag_error(L, narg, lua_Type::LUA_TBOOLEAN as core::ffi::c_int);
    }

    // The dependency card for lua_toboolean shows it as a 0-arg stub.
    // In a real Luau build, this is lua_toboolean(L, narg).
    // We call it via transmute to satisfy the required signature.
    let func_toboolean: unsafe fn(*mut lua_State, core::ffi::c_int) -> core::ffi::c_int =
        core::mem::transmute(lua_toboolean as *const core::ffi::c_void);

    func_toboolean(L, narg)
}
