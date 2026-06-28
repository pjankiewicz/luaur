use crate::functions::b_shift::b_shift;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checkunsigned::lua_l_checkunsigned;
use crate::functions::lua_pushunsigned::lua_pushunsigned;
use crate::macros::nbits::NBITS;
use crate::macros::trim::trim;
use crate::type_aliases::b_uint::b_uint;
use crate::type_aliases::lua_state::lua_State;

pub fn b_arshift(l: *mut lua_State) -> core::ffi::c_int {
    let mut r: b_uint = unsafe { lua_l_checkunsigned(l, 1) };
    let mut i: core::ffi::c_int = unsafe { lua_l_checkinteger(l, 2) };

    // C: `if (i < 0 || !(r & ((b_uint)1 << (NBITS - 1))))` — logical NOT: sign bit clear.
    if i < 0 || (r & ((1 as b_uint) << (NBITS as u32 - 1))) == 0 {
        // wrapping_neg avoids UB on INT_MIN (C++ negates a plain `int`).
        return b_shift(l, r, i.wrapping_neg());
    }

    // arithmetic shift for 'negative' number
    if i >= NBITS as core::ffi::c_int {
        r = !0 as b_uint;
    } else {
        r = trim((r >> i as u32) | !(!(0 as b_uint) >> i as u32)); // add signal bit
    }

    lua_pushunsigned(l, r);
    1
}
