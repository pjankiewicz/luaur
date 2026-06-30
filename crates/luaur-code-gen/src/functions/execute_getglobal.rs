use luaur_vm::functions::lua_v_gettable::lua_v_gettable;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::sethvalue::sethvalue;
use luaur_vm::macros::ttisstring::ttisstring;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

use crate::macros::vm_patch_c::VM_PATCH_C;
use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;

use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_c::LUAU_INSN_C;

pub unsafe fn execute_getglobal(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    let l_ptr = L as *mut luaur_vm::records::lua_state::lua_State;
    let cl = clvalue!((*(*l_ptr).ci).func);

    let mut pc_ptr = pc;
    let insn = *pc_ptr;
    pc_ptr = pc_ptr.add(1);

    let ra = VM_REG!(LUAU_INSN_A(insn) as i32, l_ptr, base);
    let aux = *pc_ptr;
    pc_ptr = pc_ptr.add(1);

    let kv = (k as *const TValue).add(aux as usize);
    LUAU_ASSERT!(ttisstring!(kv));

    // fast-path should already have been checked, so we skip checking for it here
    let h = (*cl).env;
    let slot = (LUAU_INSN_C(insn) as i32) & (*h).nodemask8 as i32;

    // slow-path, may invoke Lua calls via __index metamethod
    let mut g = TValue::default();
    sethvalue!(l_ptr, &mut g, h);
    (*l_ptr).cachedslot = slot;

    let mut current_base = base;
    vm_protect!(l_ptr, pc_ptr, current_base, {
        lua_v_gettable(l_ptr, &g as *const TValue, kv as *mut TValue, ra);
    });

    // save cachedslot to accelerate future lookups; patches currently executing instruction since pc-2 rolls back two pc++
    VM_PATCH_C(pc_ptr.offset(-2), (*l_ptr).cachedslot);

    pc_ptr
}

#[export_name = "luaur_executeGETGLOBAL"]
pub unsafe extern "C" fn executeGETGLOBAL(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_getglobal(L, pc, base, k)
}
