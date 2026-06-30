use crate::enums::lua_status::lua_Status;
use crate::functions::lua_g_pusherror::lua_g_pusherror;
use crate::records::lua_exception::lua_exception;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::pfunc::Pfunc;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_camel_case_types)]
#[export_name = "luaur_lua_d_rawrunprotected_mut"]
pub unsafe fn lua_d_rawrunprotected_mut(
    L: *mut lua_State,
    f: Pfunc,
    ud: *mut core::ffi::c_void,
) -> i32 {
    let mut status: i32 = 0;

    // Silence the default panic-hook noise for the VM's longjmp-emulation
    // unwinds (a caught `lua_exception` is a normal Lua error, not a crash).
    crate::functions::install_lua_exception_panic_hook::install_lua_exception_panic_hook();

    // In Rust, we use std::panic::catch_unwind to simulate the C++ try/catch boundary.
    // Note: This requires the 'std' library.
    let result = std::panic::catch_unwind(move || {
        if let Some(f_fn) = f {
            f_fn(L, ud);
        }
    });

    if let Err(payload) = result {
        // Check if the panic payload is a lua_exception.
        if let Some(e) = payload.downcast_ref::<lua_exception>() {
            LUAU_ASSERT!(e.getThread() == L);
            status = e.getStatus();
        } else {
            // Fallback for general panics (equivalent to catch std::exception)
            status = lua_Status::LUA_ERRRUN as i32;

            // Best-effort error message: if it's a string-like panic, we could push it,
            // but for parity with the provided skeleton and C++ catch(std::exception),
            // we attempt to push a generic or extracted message.
            let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                *s
            } else if let Some(s) = payload.downcast_ref::<alloc::string::String>() {
                s.as_str()
            } else {
                "unknown Lua error"
            };

            // We need a null-terminated string for luaG_pusherror.
            // Since we are in a panic handler and need to return a status, we use a temporary allocation.
            // Note: std::ffi::CString is used here as this function is already using std::panic.
            let temp_msg = std::ffi::CString::new(msg)
                .unwrap_or_else(|_| std::ffi::CString::new("error message contains null").unwrap());

            lua_g_pusherror(L, temp_msg.as_ptr());
        }
    }

    status
}
