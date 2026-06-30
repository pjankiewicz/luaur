use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::utf_8_decode::utf_8_decode;
use crate::macros::iscont::iscont;
use crate::macros::lua_tointeger::lua_tointeger;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_iter_aux"]
pub unsafe fn iter_aux(L: *mut lua_State) -> c_int {
    let mut len: usize = 0;
    let s = lua_l_checklstring(L, 1, &mut len);
    let mut n = lua_tointeger!(L, 2) - 1;

    if n < 0 {
        n = 0;
    } else if n < len as c_int {
        n += 1;
        while iscont(s.add(n as usize)) {
            n += 1;
        }
    }

    if n >= len as c_int {
        0
    } else {
        let mut code: i32 = 0;
        let next = utf_8_decode(s.add(n as usize), &mut code);
        if next.is_null() || iscont(next) {
            lua_l_error_l(
                L,
                c"invalid UTF-8 code".as_ptr(),
                core::format_args!("invalid UTF-8 code"),
            );
        }
        lua_pushinteger(L, n + 1);
        lua_pushinteger(L, code);
        2
    }
}
