use crate::functions::b_shift::b_shift;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::functions::lua_l_checkunsigned::lua_l_checkunsigned;
use crate::type_aliases::lua_state::lua_State;

pub fn b_rshift(l: *mut lua_State) -> core::ffi::c_int {
    // wrapping_neg avoids UB on INT_MIN (C++ negates a plain `int`); b_shift
    // treats the magnitude via unsigned_abs, so the wrapped value is handled.
    b_shift(
        l,
        lua_l_checkunsigned(l, 1),
        lua_l_checkinteger(l, 2).wrapping_neg(),
    )
}
