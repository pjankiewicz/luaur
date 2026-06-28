use crate::records::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[allow(non_snake_case)]
pub fn stacklimitreached(L: *mut lua_State, n: core::ffi::c_int) -> bool {
    unsafe {
        let stack_last = (*L).stack_last as *mut core::ffi::c_char;
        let top = (*L).top as *mut core::ffi::c_char;
        // C++ does SIGNED pointer subtraction (`ptrdiff_t`): once `top` passes
        // `stack_last`, the difference goes negative → the limit is reached. The
        // original `as usize` subtraction underflowed when `top > stack_last` —
        // a panic with overflow-checks (fuzz build), and in release it wraps to a
        // huge value, wrongly reporting "limit NOT reached". Compute it signed to
        // match C++. (Found by the `splice` fuzz target — a deeply recursive
        // program pushes `top` past `stack_last`.)
        let diff = (stack_last as isize) - (top as isize);
        let threshold = (n as isize) * (core::mem::size_of::<TValue>() as isize);
        diff <= threshold
    }
}
