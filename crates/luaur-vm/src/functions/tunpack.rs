use crate::enums::lua_type::lua_Type;
use crate::functions::lua_checkstack::lua_checkstack;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_objlen::lua_objlen;
use crate::functions::lua_rawgeti::lua_rawgeti;
use crate::macros::hvalue::hvalue;
use crate::macros::setobj_2_s::setobj2s;
use crate::type_aliases::lua_state::lua_State;

#[no_mangle]
pub unsafe fn tunpack(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    let t = hvalue!((*L).base);

    let i = lua_l_optinteger(L, 2, 1);
    let e = lua_objlen(L, 1);
    let e = lua_l_optinteger(L, 3, e);

    if i > e {
        return 0; // empty range
    }

    // `n` here is the element count MINUS ONE. C++ guards on this value
    // (`n >= INT_MAX`) BEFORE adding one, so a full-range request
    // (i = INT_MIN, e = INT_MAX -> n = 0xFFFF_FFFF) is rejected. Adding one first
    // (as the previous port did) wrapped n to 0, passed the guard, and let the
    // push loop overrun the stack into an api_incr_top assert (SIGTRAP).
    let n = (e as u32).wrapping_sub(i as u32); // number of elements minus 1 (avoid overflows)
    if n >= core::ffi::c_int::MAX as u32
        || lua_checkstack(L, n.wrapping_add(1) as core::ffi::c_int) == 0
    {
        lua_l_error_l(
            L,
            c"too many results to unpack".as_ptr(),
            core::format_args!("too many results to unpack"),
        );
    }
    let n = n + 1; // safe: guard above guarantees n (minus one) < INT_MAX

    // fast-path: direct array-to-stack copy
    if i == 1 && (n as core::ffi::c_int) <= (*t).sizearray {
        for i_idx in 0..(n as core::ffi::c_int) {
            let src = (*t).array.add(i_idx as usize);
            let dst = (*L).top.offset(i_idx as isize);
            setobj2s!(L, dst, src);
        }
        (*L).top = (*L).top.offset(n as isize);
    } else {
        // push arg[i..e - 1] (to avoid overflows)
        let mut current_i = i;
        while current_i < e {
            lua_rawgeti(L, 1, current_i);
            current_i += 1;
        }
        lua_rawgeti(L, 1, e); // push last element
    }

    n as core::ffi::c_int
}
