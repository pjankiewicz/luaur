use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_vm::macros::cast_int::cast_int;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::setnilvalue::setnilvalue;
use luaur_vm::macros::setobj_2_s::setobj_2_s as setobj2s;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

use crate::macros::vm_protect::vm_protect as VM_PROTECT;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;

pub unsafe fn execute_prepvarargs(
    L: *mut lua_State,
    pc: *const Instruction,
    mut base: StkId,
    _k: *mut TValue,
) -> *const Instruction {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;
    let cl = clvalue!((*(*l_ptr).ci).func);
    let mut pc_ptr = pc;
    let insn = *pc_ptr;
    pc_ptr = pc_ptr.add(1);
    let numparams = LUAU_INSN_A(insn) as i32;

    // all fixed parameters are copied after the top so we need more stack space
    VM_PROTECT!(l_ptr, pc_ptr as *const u32, base, {
        let n = (*cl).stacksize as i32 + numparams;
        if luaur_vm::macros::stacklimitreached::stacklimitreached(l_ptr, n) {
            luaur_vm::functions::lua_d_growstack::lua_d_growstack(l_ptr, n);
        } else {
            // condhardstacktests expansion:
            // In Luau, condhardstacktests is usually gated by a debug flag or internal check.
            // Since LuauCheckStackVariable was not found, we use the standard VM logic
            // which often omits the hard stack test in production or uses a different flag.
            // We'll follow the luaD_checkstack macro logic but skip the missing FFlag.
        }
    });

    LUAU_ASSERT!(cast_int!((*l_ptr).top.offset_from(base)) >= numparams);

    // move fixed parameters to final position
    let fixed = base; // first fixed argument
    let new_base = (*l_ptr).top; // final position of first argument

    for i in 0..numparams {
        setobj2s!(l_ptr, new_base.add(i as usize), fixed.add(i as usize));
        setnilvalue!(fixed.add(i as usize));
    }

    // rewire our stack frame to point to the new base
    (*(*l_ptr).ci).base = new_base;
    (*(*l_ptr).ci).top = new_base.add((*cl).stacksize as usize);

    (*l_ptr).base = new_base;
    (*l_ptr).top = (*(*l_ptr).ci).top;

    pc_ptr
}

#[export_name = "luaur_executePREPVARARGS"]
pub unsafe extern "C" fn executePREPVARARGS(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_prepvarargs(L, pc, base, k)
}
