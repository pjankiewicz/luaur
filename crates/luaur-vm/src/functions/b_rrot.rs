use crate::functions::b_rot::b_rot;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::type_aliases::lua_state::lua_State;

pub fn b_rrot(l: *mut lua_State) -> core::ffi::c_int {
    // wrapping_neg avoids UB on INT_MIN (C++ negates a plain `int`); b_rot masks
    // the count with `& (NBITS-1)`, so the wrapped value rotates correctly.
    b_rot(l, lua_l_checkinteger(l, 2).wrapping_neg())
}
