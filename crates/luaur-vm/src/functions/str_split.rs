use crate::functions::lua_createtable::lua_createtable;
use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_optlstring::lua_l_optlstring;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushlstring::lua_pushlstring;
use crate::functions::lua_settable::lua_settable;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

#[export_name = "luaur_str_split"]
pub unsafe fn str_split(l: *mut lua_State) -> c_int {
    extern "C" {
        fn memcmp(s1: *const core::ffi::c_void, s2: *const core::ffi::c_void, n: usize) -> c_int;
    }

    let mut haystack_len: usize = 0;
    let haystack = lua_l_checklstring(l, 1, &mut haystack_len);
    let mut needle_len: usize = 0;
    let needle = lua_l_optlstring(l, 2, c",".as_ptr() as *const c_char, &mut needle_len);

    let begin = haystack;
    let end = haystack.add(haystack_len);
    let mut span_start = begin;
    let mut num_matches = 0;

    lua_createtable(l, 0, 0);

    let mut iter = begin;
    if needle_len == 0 {
        iter = iter.add(1);
    }

    // Don't iterate the last needleLen - 1 bytes of the string - they are
    // impossible to be splits and would let us memcmp past the end of the
    // buffer.
    while iter <= end.offset(-(needle_len as isize)) {
        // Use of memcmp here instead of strncmp is so that we allow embedded
        // nulls to be used in either of the haystack or the needle strings.
        if memcmp(
            iter as *const core::ffi::c_void,
            needle as *const core::ffi::c_void,
            needle_len,
        ) == 0
        {
            num_matches += 1;
            lua_pushinteger(l, num_matches);
            lua_pushlstring(l, span_start, iter.offset_from(span_start) as usize);
            lua_settable(l, -3);

            span_start = iter.add(needle_len);
            if needle_len > 0 {
                iter = iter.add(needle_len - 1);
            }
        }
        iter = iter.add(1);
    }

    if needle_len > 0 {
        num_matches += 1;
        lua_pushinteger(l, num_matches);
        lua_pushlstring(l, span_start, end.offset_from(span_start) as usize);
        lua_settable(l, -3);
    }

    1
}
