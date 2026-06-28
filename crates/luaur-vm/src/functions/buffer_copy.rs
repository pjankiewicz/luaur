use crate::functions::lua_l_checkbuffer::lua_l_checkbuffer;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::macros::isoutofbounds::isoutofbounds;
use crate::macros::lua_l_error::luaL_error;
use crate::type_aliases::lua_state::lua_State;

pub fn buffer_copy(L: *mut lua_State) -> core::ffi::c_int {
    let mut tlen: usize = 0;
    let tbuf = lua_l_checkbuffer(L, 1, &mut tlen);
    let toffset = lua_l_checkinteger(L, 2);

    let mut slen: usize = 0;
    let sbuf = lua_l_checkbuffer(L, 3, &mut slen);
    let soffset = lua_l_optinteger(L, 4, 0);

    // C++ evaluates `int(slen) - soffset` as the default eagerly (signed overflow
    // is UB upstream for soffset = INT_MIN); wrapping_sub reproduces the two's-
    // complement value C++ relies on, which the `size < 0` / isoutofbounds checks
    // below then reject. (Upstream UBSan: lbuflib.cpp:257.)
    let size = lua_l_optinteger(L, 5, (slen as core::ffi::c_int).wrapping_sub(soffset));

    if size < 0 {
        luaL_error!(L, "buffer access out of bounds");
    }

    if isoutofbounds(soffset, slen, size as usize) {
        luaL_error!(L, "buffer access out of bounds");
    }

    if isoutofbounds(toffset, tlen, size as usize) {
        luaL_error!(L, "buffer access out of bounds");
    }

    unsafe {
        core::ptr::copy(
            (sbuf as *const core::ffi::c_char).offset(soffset as isize),
            (tbuf as *mut core::ffi::c_char).offset(toffset as isize),
            size as usize,
        );
    }

    0
}
