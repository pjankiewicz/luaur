use crate::functions::lua_pushunsigned::lua_pushunsigned;
use crate::macros::nbits::NBITS;
use crate::macros::trim::trim;
use crate::type_aliases::b_uint::b_uint;
use crate::type_aliases::lua_state::lua_State;

pub fn b_shift(l: *mut lua_State, mut r: b_uint, mut i: core::ffi::c_int) -> core::ffi::c_int {
    // Mirrors VM/src/lbitlib.cpp:b_shift
    if i < 0 {
        // Magnitude of the (right) shift. `i.unsigned_abs()` is defined for
        // `i == INT_MIN` (the C++ `i = -i` is UB there), and using it as the
        // bound also avoids a shift-by->=32 (itself UB) — |i| >= NBITS yields 0.
        let amount = i.unsigned_abs();
        r = trim(r);
        if amount >= NBITS as u32 {
            r = 0;
        } else {
            r >>= amount;
        }
    } else {
        if i >= NBITS as core::ffi::c_int {
            r = 0;
        } else {
            r <<= i as u32;
        }
        r = trim(r);
    }

    lua_pushunsigned(l, r);
    1
}
