use crate::macros::api_check::api_check;
use crate::macros::api_checknelems::api_checknelems;
use crate::macros::blackbit::BLACKBIT;
use crate::macros::setobj_2_s::setobj_2_s;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_xmove"]
pub unsafe fn lua_xmove(from: *mut lua_State, to: *mut lua_State, n: core::ffi::c_int) {
    api_check!(from, n >= 0);

    if from == to {
        return;
    }

    api_checknelems!(from, n);
    api_check!(from, (*from).global == (*to).global);
    api_check!(from, (*(*to).ci).top.offset_from((*to).top) >= n as isize);

    // Manual inline of lua_c_threadbarrier!(to) to bypass broken macros
    let marked = (*to).hdr.marked as i32;
    if (marked & (1 << BLACKBIT)) != 0 {
        crate::functions::lua_c_barrierback::lua_c_barrierback(to, to as *mut _, &mut (*to).gclist);
    }

    let ttop = (*to).top;
    let ftop = (*from).top.offset(-(n as isize));

    for i in 0..n {
        setobj_2_s!(to, ttop.offset(i as isize), ftop.offset(i as isize));
    }

    (*from).top = ftop;
    (*to).top = ttop.offset(n as isize);
}
