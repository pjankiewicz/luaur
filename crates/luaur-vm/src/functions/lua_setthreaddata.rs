use crate::records::lua_state::lua_State;
use core::ffi::c_void;

#[export_name = "luaur_lua_setthreaddata"]
pub unsafe fn lua_setthreaddata(l: *mut lua_State, data: *mut c_void) {
    (*l).userdata = data;
}
