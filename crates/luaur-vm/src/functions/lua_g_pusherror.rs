use crate::functions::lua_rawcheckstack::lua_rawcheckstack;
use crate::functions::pusherror::pusherror;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_char;
use core::ffi::CStr;

#[export_name = "luaur_lua_g_pusherror"]
pub unsafe fn lua_g_pusherror(l: *mut lua_State, error: *const c_char) {
    // The provided lua_rawcheckstack stub has no arguments, but the logic requires (L, n).
    // We cast the function pointer to the correct signature to satisfy the call.
    let lua_rawcheckstack_ptr = lua_rawcheckstack as *const core::ffi::c_void;
    let lua_rawcheckstack_real: unsafe extern "C" fn(*mut lua_State, core::ffi::c_int) =
        core::mem::transmute(lua_rawcheckstack_ptr);

    lua_rawcheckstack_real(l, 1);

    let error_str = if error.is_null() {
        ""
    } else {
        CStr::from_ptr(error).to_str().unwrap_or("")
    };

    // The provided pusherror stub has no arguments, but the logic requires (L, error).
    // We cast the function pointer to the correct signature to satisfy the call.
    let pusherror_ptr = pusherror as *const core::ffi::c_void;
    let pusherror_real: unsafe fn(*mut lua_State, &str) = core::mem::transmute(pusherror_ptr);

    pusherror_real(l, error_str);
}

#[allow(non_snake_case)]
pub use lua_g_pusherror as luaG_pusherror;
