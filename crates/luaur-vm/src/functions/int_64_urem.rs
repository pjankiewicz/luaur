use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_urem"]
pub unsafe fn int64_urem(l: *mut LuaState) -> core::ffi::c_int {
    let a = lua_l_checkinteger_64(l, 1) as u64;
    let b = lua_l_checkinteger_64(l, 2) as u64;

    if b == 0 {
        // luaL_error macro expects a *const c_char format parameter;
        // this translation avoids the macro and calls the underlying function directly.
        crate::functions::lua_l_error_l::lua_l_error_l(
            l,
            c"division by zero".as_ptr(),
            core::format_args!("division by zero"),
        );
    }

    lua_pushinteger_64(l, (a % b) as i64);

    1
}
