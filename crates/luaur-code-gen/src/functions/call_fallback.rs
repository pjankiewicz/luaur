use core::ffi::{c_int, c_void};

use crate::macros::call_fallback_yield::CALL_FALLBACK_YIELD;
use luaur_vm::functions::lua_v_tryfunc_tm::lua_v_tryfunc_tm;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::lua_callinfo_native::LUA_CALLINFO_NATIVE;
use luaur_vm::macros::lua_multret::LUA_MULTRET;
use luaur_vm::macros::setnilvalue::setnilvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::macros::ttisfunction::ttisfunction;
use luaur_vm::records::call_info::CallInfo;
use luaur_vm::records::closure::Closure;
use luaur_vm::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

pub unsafe fn call_fallback(
    L: *mut lua_State,
    ra: StkId,
    mut argtop: StkId,
    nresults: c_int,
) -> *mut Closure {
    if !ttisfunction!(ra as *const TValue) {
        lua_v_tryfunc_tm(L, ra);
        argtop = argtop.add(1);
    }

    let ccl = clvalue!(ra as *const TValue);

    if luaur_common::FFlag::LuauClosureUsageCounter.get() {
        (*ccl).usage += 1;
    }

    let ci = incr_ci_local(L);
    (*ci).func = ra;
    (*ci).base = ra.add(1);
    (*ci).top = argtop.add((*ccl).stacksize as usize);
    (*ci).savedpc = core::ptr::null();
    (*ci).flags = 0;
    (*ci).nresults = nresults;

    (*L).base = (*ci).base;
    (*L).top = argtop;

    luaur_vm::macros::lua_d_checkstackfornewci::luaD_checkstackfornewci(
        L,
        (*ccl).stacksize as c_int,
    );
    luaur_common::LUAU_ASSERT!((*ci).top <= (*L).stack_last);

    if (*ccl).isC == 0 {
        let p = {
            let l = &(*ccl).inner.l;
            l.p
        };

        let mut argi = (*L).top;
        let argend = (*L).base.add((*p).numparams as usize);
        while argi < argend {
            setnilvalue!(argi);
            argi = argi.add(1);
        }
        (*L).top = if (*p).is_vararg != 0 { argi } else { (*ci).top };

        (*ci).savedpc = (*p).code;

        let has_native_target = if luaur_common::FFlag::LuauNativeCodeTargetCheck.get() {
            (*p).exectarget != 0
        } else {
            !(*p).execdata.is_null()
        };

        if has_native_target {
            (*ci).flags = LUA_CALLINFO_NATIVE as u32;
        }

        ccl
    } else {
        let func = {
            let c = &(*ccl).inner.c;
            c.f
        };
        let n = match func {
            Some(f) => f(L),
            None => 0,
        };

        if n < 0 {
            return CALL_FALLBACK_YIELD as usize as *mut Closure;
        }

        let ci = (*L).ci;
        let cip = ci.sub(1);

        if luaur_common::FFlag::LuauClosureUsageCounter.get() {
            luaur_common::LUAU_ASSERT!((*ccl).usage > 0);
            (*ccl).usage -= 1;
        }

        let mut res = (*ci).func;
        let mut vali = (*L).top.sub(n as usize);
        let valend = (*L).top;

        let mut i = nresults;
        while i != 0 && vali < valend {
            setobj2s!(L, res, vali as *const TValue);
            res = res.add(1);
            vali = vali.add(1);
            i -= 1;
        }
        while i > 0 {
            setnilvalue!(res);
            res = res.add(1);
            i -= 1;
        }

        (*L).ci = cip;
        (*L).base = (*cip).base;
        (*L).top = if nresults == LUA_MULTRET {
            res
        } else {
            (*cip).top
        };

        core::ptr::null_mut()
    }
}

unsafe fn incr_ci_local(L: *mut lua_State) -> *mut CallInfo {
    if (*L).ci == (*L).end_ci {
        luaur_vm::functions::lua_d_grow_ci::luaD_growCI(L);
    } else {
        (*L).ci = (*L).ci.add(1);
    }

    (*L).ci
}

#[export_name = "luaur_callFallback"]
pub unsafe extern "C" fn callFallback(
    L: *mut lua_State,
    ra: StkId,
    argtop: StkId,
    nresults: c_int,
) -> *mut c_void {
    call_fallback(L, ra, argtop, nresults).cast()
}
