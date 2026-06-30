use luaur_vm::functions::lua_h_resizearray::lua_h_resizearray;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::hvalue::hvalue;
use luaur_vm::macros::lua_c_barrierfast::lua_c_barrierfast;
use luaur_vm::macros::setobj_2_t::setobj2t;
use luaur_vm::macros::ttistable::ttistable;
use luaur_vm::type_aliases::t_value::TValue;

use crate::macros::vm_protect_pc::VM_PROTECT_PC;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::stk_id::StkId;

use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_b::LUAU_INSN_B;
use luaur_common::macros::luau_insn_c::LUAU_INSN_C;
use luaur_vm::macros::lua_multret::LUA_MULTRET;

pub unsafe fn execute_setlist(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    _k: *mut TValue,
) -> *const Instruction {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;
    let cl = clvalue!((*(*l_ptr).ci).func);
    let _ = cl; // unused in this function

    let mut pc_ptr = pc;
    let insn = *pc_ptr;
    pc_ptr = pc_ptr.add(1);

    let ra = VM_REG!(LUAU_INSN_A(insn) as i32, l_ptr, base);
    // note: this can point to L->top if c == LUA_MULTRET making VM_REG unsafe to use
    let rb = base.add(LUAU_INSN_B(insn) as usize);
    let mut c = (LUAU_INSN_C(insn) as i32) - 1;
    let index = *pc_ptr;
    pc_ptr = pc_ptr.add(1);

    if c == LUA_MULTRET {
        c = (*l_ptr).top.offset_from(rb) as i32;
        (*l_ptr).top = (*(*l_ptr).ci).top;
    }

    let h = hvalue!(ra as *const TValue);

    // TODO: we really don't need this anymore
    if !ttistable!(ra as *const TValue) {
        return core::ptr::null(); // temporary workaround to weaken a rather powerful exploitation primitive in case of a MITM attack on bytecode
    }

    let last = index as i32 + c - 1;
    if last > (*h).sizearray {
        // VM_PROTECT_PC expects the local crate's lua_State pointer
        VM_PROTECT_PC(L, pc_ptr as *const u32); // luaH_resizearray may fail due to OOM

        lua_h_resizearray(l_ptr, h, last);
    }

    let array = (*h).array;

    for i in 0..c {
        setobj2t!(
            l_ptr,
            array.add((index as i32 + i - 1) as usize),
            rb.add(i as usize)
        );
    }

    lua_c_barrierfast!(l_ptr, h);
    pc_ptr
}

#[export_name = "luaur_executeSETLIST"]
pub unsafe extern "C" fn executeSETLIST(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_setlist(L, pc, base, k)
}
