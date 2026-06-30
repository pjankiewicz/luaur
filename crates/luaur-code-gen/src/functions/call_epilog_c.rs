use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::setnilvalue::setnilvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

use crate::type_aliases::lua_state::lua_State;

#[allow(non_snake_case)]
pub const LUA_MULTRET: i32 = luaur_vm::macros::lua_multret::LUA_MULTRET;

pub unsafe fn call_epilog_c(L: *mut lua_State, nresults: i32, n: i32) {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;

    // ci is our callinfo, cip is our parent
    let ci = (*l_ptr).ci;
    let cip = ci.offset(-1);

    // copy return values into parent stack (but only up to nresults!), fill the rest with nil
    // note: in MULTRET context nresults starts as -1 so i != 0 condition never activates intentionally
    let mut res: StkId = (*ci).func;
    let mut vali: StkId = (*l_ptr).top.offset(-n as isize);
    let valend: StkId = (*l_ptr).top;

    let mut i: i32 = nresults;
    while i != 0 && vali < valend {
        setobj2s!(l_ptr, res, vali);
        res = res.add(1);
        vali = vali.add(1);
        i -= 1;
    }

    while i > 0 {
        setnilvalue!(res);
        res = res.add(1);
        i -= 1;
    }

    // pop the stack frame
    (*l_ptr).ci = cip;
    (*l_ptr).base = (*cip).base;
    (*l_ptr).top = if nresults == LUA_MULTRET {
        res
    } else {
        (*cip).top
    };
}

#[export_name = "luaur_callEpilogC"]
pub unsafe extern "C" fn callEpilogC(L: *mut lua_State, nresults: i32, n: i32) {
    call_epilog_c(L, nresults, n);
}
