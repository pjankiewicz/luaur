use core::ffi::{c_int, c_void};

use luaur_vm::functions::lua_v_tryfunc_tm::lua_v_tryfunc_tm;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::ttisfunction::ttisfunction;
use luaur_vm::records::call_info::CallInfo;
use luaur_vm::records::closure::Closure;
use luaur_vm::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

pub unsafe fn call_prolog(
    L: *mut lua_State,
    ra: *mut TValue,
    mut argtop: StkId,
    nresults: c_int,
) -> *mut Closure {
    if !ttisfunction!(ra as *const TValue) {
        lua_v_tryfunc_tm(L, ra);
        argtop = argtop.add(1);
    }

    let ccl = clvalue!(ra as *const TValue);

    let ci = incr_ci_local(L);
    (*ci).func = ra;
    (*ci).base = ra.add(1);
    (*ci).top = argtop.add((*ccl).stacksize as usize);
    (*ci).savedpc = core::ptr::null();
    (*ci).flags = 0;
    (*ci).nresults = nresults;

    if luaur_common::FFlag::LuauClosureUsageCounter.get() {
        (*ccl).usage += 1;
    }

    (*L).base = (*ci).base;
    (*L).top = argtop;

    luaur_vm::macros::lua_d_checkstackfornewci::luaD_checkstackfornewci(
        L,
        (*ccl).stacksize as c_int,
    );

    ccl
}

unsafe fn incr_ci_local(L: *mut lua_State) -> *mut CallInfo {
    if (*L).ci == (*L).end_ci {
        luaur_vm::functions::lua_d_grow_ci::luaD_growCI(L);
    } else {
        (*L).ci = (*L).ci.add(1);
    }

    (*L).ci
}

#[export_name = "luaur_callProlog"]
pub unsafe extern "C" fn callProlog(
    L: *mut lua_State,
    ra: *mut TValue,
    argtop: StkId,
    nresults: c_int,
) -> *mut c_void {
    call_prolog(L, ra, argtop, nresults).cast()
}
