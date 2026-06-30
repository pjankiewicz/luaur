use core::ffi::c_int;

use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::setnilvalue::setnilvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

pub unsafe fn execute_getvarargs_const(L: *mut lua_State, base: StkId, rai: c_int, b: c_int) {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;
    let cl = clvalue!((*(*l_ptr).ci).func);
    let p = {
        let l = &(*cl).inner.l;
        l.p
    };
    let n = base.offset_from((*(*l_ptr).ci).func) as c_int - (*p).numparams as c_int - 1;

    let ra = VM_REG!(rai, l_ptr, base) as *mut TValue;

    let mut j = 0;
    while j < b && j < n {
        setobj2s!(
            l_ptr,
            ra.add(j as usize),
            base.sub(n as usize).add(j as usize) as *const TValue
        );
        j += 1;
    }
    let mut j = n;
    while j < b {
        setnilvalue!(ra.add(j as usize));
        j += 1;
    }
}

#[export_name = "luaur_executeGETVARARGSConst"]
pub unsafe extern "C" fn executeGETVARARGSConst(
    L: *mut lua_State,
    base: StkId,
    rai: c_int,
    b: c_int,
) {
    execute_getvarargs_const(L, base, rai, b);
}
