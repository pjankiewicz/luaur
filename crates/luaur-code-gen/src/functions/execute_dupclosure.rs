use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_protect_pc::VM_PROTECT_PC;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::enums::luau_capture_type::LuauCaptureType;
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_b::LUAU_INSN_B;
use luaur_common::macros::luau_insn_d::LUAU_INSN_D;
use luaur_common::macros::luau_insn_op::LUAU_INSN_OP;
use luaur_vm::functions::lua_f_new_lclosure::lua_f_new_lclosure;
use luaur_vm::functions::lua_o_rawequal_obj::luaO_rawequalObj;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::lua_c_barrier::luaC_barrier;
use luaur_vm::macros::lua_c_check_gc::luaC_checkGC;
use luaur_vm::macros::setclvalue::setclvalue;
use luaur_vm::macros::setobj::setobj;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;

pub unsafe fn execute_dupclosure(
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
    let kv = k.add(LUAU_INSN_D(insn) as usize);

    let kcl = clvalue!(kv as *const TValue);

    VM_PROTECT_PC(l_ptr, pc_ptr);

    let mut ncl = if (*kcl).env == (*cl).env {
        kcl
    } else {
        let kp = {
            let l = &(*kcl).inner.l;
            l.p
        };
        lua_f_new_lclosure(l_ptr, (*kcl).nupvalues as i32, (*cl).env, kp)
    };
    setclvalue!(l_ptr, ra, ncl);

    let mut ui: i32 = 0;
    while ui < (*kcl).nupvalues as i32 {
        let uinsn = *pc_ptr.add(ui as usize);
        luaur_common::LUAU_ASSERT!(LUAU_INSN_OP(uinsn) == LuauOpcode::LOP_CAPTURE as u32);
        luaur_common::LUAU_ASSERT!(
            LUAU_INSN_A(uinsn) == LuauCaptureType::LCT_VAL as u32
                || LUAU_INSN_A(uinsn) == LuauCaptureType::LCT_UPVAL as u32
        );

        let uv: *mut TValue = if LUAU_INSN_A(uinsn) == LuauCaptureType::LCT_VAL as u32 {
            VM_REG!(LUAU_INSN_B(uinsn), l_ptr, base) as *mut TValue
        } else {
            let l = &mut (*cl).inner.l;
            l.uprefs.as_mut_ptr().add(LUAU_INSN_B(uinsn) as usize)
        };

        let uref = {
            let l = &mut (*ncl).inner.l;
            l.uprefs.as_mut_ptr().add(ui as usize)
        };

        if ncl == kcl && luaO_rawequalObj(uref as *const TValue, uv as *const TValue) != 0 {
            ui += 1;
            continue;
        }

        if ncl == kcl && (*kcl).preload == 0 {
            let kp = {
                let l = &(*kcl).inner.l;
                l.p
            };
            ncl = lua_f_new_lclosure(l_ptr, (*kcl).nupvalues as i32, (*cl).env, kp);
            setclvalue!(l_ptr, ra, ncl);

            ui = 0;
            continue;
        }

        setobj!(l_ptr, uref, uv as *const TValue);
        luaC_barrier!(l_ptr, ncl, uv as *const TValue);
        ui += 1;
    }

    (*ncl).preload = 0;

    if kcl != ncl {
        let mut current_base = base;
        vm_protect!(l_ptr, pc_ptr, current_base, {
            luaC_checkGC!(l_ptr);
        });
    }

    pc_ptr.add((*kcl).nupvalues as usize)
}

#[export_name = "luaur_executeDUPCLOSURE"]
pub unsafe extern "C" fn executeDUPCLOSURE(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_dupclosure(L, pc, base, k)
}
