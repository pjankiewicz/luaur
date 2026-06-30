use crate::macros::lua_buffersize::LUA_BUFFERSIZE;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_char;

#[export_name = "luaur_lua_l_buffinit"]
#[allow(non_snake_case)]
pub unsafe fn lua_l_buffinit(L: *mut lua_State, B: *mut LuaLStrbuf) {
    // start with an internal buffer
    (*B).p = (*B).buffer.as_mut_ptr() as *mut c_char;
    (*B).end = (*B).p.wrapping_add(LUA_BUFFERSIZE as usize);

    (*B).L = L;
    (*B).storage = core::ptr::null_mut();
}
