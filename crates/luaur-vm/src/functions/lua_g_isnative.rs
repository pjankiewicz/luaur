use crate::macros::lua_callinfo_native::LUA_CALLINFO_NATIVE;
use crate::records::call_info::CallInfo;
use crate::records::lua_state::lua_State;
use crate::type_aliases::lua_state::lua_State as LuaState;

#[export_name = "luaur_luaG_isnative"]
pub unsafe fn luaG_isnative(L: *mut lua_State, level: core::ffi::c_int) -> core::ffi::c_int {
    if (level as u32) >= ((unsafe { (*L).ci }).offset_from(unsafe { (*L).base_ci }) as u32) {
        return 0;
    }

    let ci = unsafe { (*L).ci.offset(-level as isize) };
    if (unsafe { (*ci).flags } & LUA_CALLINFO_NATIVE as u32) != 0 {
        1
    } else {
        0
    }
}
