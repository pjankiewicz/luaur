use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_checkstack::lua_l_checkstack;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::u_posrelat::u_posrelat;
use crate::functions::utf_8_decode::utf_8_decode;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[export_name = "luaur_codepoint"]
pub unsafe fn codepoint(L: *mut lua_State) -> c_int {
    let mut len: usize = 0;
    let mut s = lua_l_checklstring(L, 1, &mut len);

    let posi = u_posrelat(lua_l_optinteger(L, 2, 1), len);
    let pose = u_posrelat(lua_l_optinteger(L, 3, posi), len);

    luaL_argcheck!(L, posi >= 1, 2, "out of range");
    luaL_argcheck!(L, pose <= len as i32, 3, "out of range");

    if posi > pose {
        return 0; // empty interval; return no values
    }

    if (pose - posi) >= c_int::MAX as i32 {
        lua_l_error_l(
            L,
            c"string slice too long".as_ptr(),
            core::format_args!("string slice too long"),
        );
    }

    let n = (pose - posi) + 1;
    lua_l_checkstack(L, n, "string slice too long");

    let mut n = 0;
    let se = s.add(pose as usize);
    s = s.add((posi - 1) as usize);

    while s < se {
        let mut code: i32 = 0;
        s = utf_8_decode(s, &mut code);
        if s.is_null() {
            lua_l_error_l(
                L,
                c"invalid UTF-8 code".as_ptr(),
                core::format_args!("invalid UTF-8 code"),
            );
        }
        lua_pushinteger(L, code);
        n += 1;
    }

    n
}
