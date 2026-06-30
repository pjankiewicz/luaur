use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_buffinitsize::lua_l_buffinitsize;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_pushresultsize::lua_l_pushresultsize;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::macros::uchar::uchar;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

#[export_name = "luaur_str_char"]
pub unsafe fn str_char(L: *mut lua_State) -> c_int {
    let n = lua_gettop(L); // number of arguments

    let mut b = LuaLStrbuf {
        p: core::ptr::null_mut(),
        end: core::ptr::null_mut(),
        L: core::ptr::null_mut(),
        storage: core::ptr::null_mut(),
        buffer: [0; 512],
    };
    let ptr = lua_l_buffinitsize(L, &mut b as *mut LuaLStrbuf, n as usize);

    let mut i = 1;
    while i <= n {
        let c = lua_l_checkinteger(L, i as c_int);
        luaL_argcheck!(
            L,
            i32::from(uchar(c)) == c as c_int,
            i as c_int,
            "invalid value"
        );

        *ptr.offset((i - 1) as isize) = uchar(c) as c_char;
        i += 1;
    }
    lua_l_pushresultsize(&mut b as *mut LuaLStrbuf, n as usize);
    1
}
