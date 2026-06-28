use crate::functions::lua_l_checkbuffer::lua_l_checkbuffer;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checkunsigned::lua_l_checkunsigned;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::macros::isoutofbounds::isoutofbounds;
use crate::macros::lua_l_error::luaL_error;
use crate::type_aliases::lua_state::lua_State;

pub fn buffer_fill(L: *mut lua_State) -> core::ffi::c_int {
    let mut len: usize = 0;
    let buf = lua_l_checkbuffer(L, 1, &mut len);
    let offset = lua_l_checkinteger(L, 2);
    let value = lua_l_checkunsigned(L, 3);
    // C++ evaluates `int(len) - offset` as the default eagerly (signed overflow
    // is UB upstream for offset = INT_MIN); wrapping_sub reproduces the two's-
    // complement value C++ relies on, which the `size < 0` / isoutofbounds checks
    // below then reject. (Upstream UBSan: lbuflib.cpp:278.)
    let size = lua_l_optinteger(L, 4, (len as core::ffi::c_int).wrapping_sub(offset));

    if size < 0 {
        luaL_error!(L, "buffer access out of bounds");
    }

    if isoutofbounds(offset, len, size as usize) {
        luaL_error!(L, "buffer access out of bounds");
    }

    unsafe {
        core::ptr::write_bytes(
            (buf as *mut core::ffi::c_char).offset(offset as isize),
            (value & 0xff) as u8,
            size as usize,
        );
    }

    0
}
