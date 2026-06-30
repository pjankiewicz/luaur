use core::ffi::c_int;

use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::functions::lua_d_growstack::lua_d_growstack;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::macros::stacklimitreached::stacklimitreached;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

pub unsafe fn execute_getvarargs_mult_ret(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    rai: c_int,
) {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;
    let cl = clvalue!((*(*l_ptr).ci).func);
    let p = {
        let l = &(*cl).inner.l;
        l.p
    };
    let n = base.offset_from((*(*l_ptr).ci).func) as c_int - (*p).numparams as c_int - 1;

    let mut current_base = base;
    vm_protect!(l_ptr, pc, current_base, {
        if stacklimitreached(l_ptr, n) {
            lua_d_growstack(l_ptr, n);
        }
    });

    let ra = VM_REG!(rai, l_ptr, current_base) as *mut TValue;
    for j in 0..n {
        setobj2s!(
            l_ptr,
            ra.add(j as usize),
            current_base.sub(n as usize).add(j as usize) as *const TValue
        );
    }

    (*l_ptr).top = ra.add(n as usize);
}

#[export_name = "luaur_executeGETVARARGSMultRet"]
pub unsafe extern "C" fn executeGETVARARGSMultRet(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    rai: c_int,
) {
    execute_getvarargs_mult_ret(L, pc, base, rai);
}
