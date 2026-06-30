use crate::macros::vm_patch_c::VM_PATCH_C;
use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_protect_pc::VM_PROTECT_PC;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_aux_kv_16::LUAU_INSN_AUX_KV16;
use luaur_common::macros::luau_insn_b::LUAU_INSN_B;
use luaur_common::macros::luau_insn_c::LUAU_INSN_C;
use luaur_common::macros::luau_insn_op::LUAU_INSN_OP;
use luaur_vm::functions::lua_h_setstr::lua_h_setstr as luaH_setstr;
use luaur_vm::functions::lua_v_call_tm::lua_v_call_tm;
use luaur_vm::functions::lua_v_settable::lua_v_settable;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::fastnotm::fastnotm;
use luaur_vm::macros::fasttm::fasttm;
use luaur_vm::macros::gval_2_slot::gval2slot;
use luaur_vm::macros::hvalue::hvalue;
use luaur_vm::macros::lua_c_barriert::luaC_barriert;
use luaur_vm::macros::setobj_2_s::setobj_2_s as setobj2s;
use luaur_vm::macros::setobj_2_t::setobj2t;
use luaur_vm::macros::tsvalue::tsvalue;
use luaur_vm::macros::ttisfunction::ttisfunction;
use luaur_vm::macros::ttisstring::ttisstring;
use luaur_vm::macros::ttistable::ttistable;
use luaur_vm::macros::ttisuserdata::ttisuserdata;
use luaur_vm::macros::uvalue::uvalue;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;
use luaur_vm::type_aliases::tms::TMS;

#[inline]
unsafe fn vm_kv(
    i: u32,
    cl: *mut luaur_vm::records::closure::Closure,
    k: *mut TValue,
) -> *mut TValue {
    let p = {
        let l = &(*cl).inner.l;
        l.p
    };
    luaur_common::LUAU_ASSERT!(i < (*p).sizek as u32);
    k.add(i as usize)
}

pub unsafe fn execute_settableks(
    L: *mut lua_State,
    pc: *const Instruction,
    mut base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    let cl = clvalue!((*(*L).ci).func);
    let mut pc_ptr = pc;
    let insn = *pc_ptr;
    pc_ptr = pc_ptr.add(1);
    let op = LUAU_INSN_OP(insn);
    let ra = VM_REG!(LUAU_INSN_A(insn) as i32, L, base) as *mut TValue;
    let rb = VM_REG!(LUAU_INSN_B(insn) as i32, L, base) as *mut TValue;
    let aux = *pc_ptr;
    pc_ptr = pc_ptr.add(1);
    let kv = vm_kv(
        if op == LuauOpcode::LOP_SETUDATAKS as u32 {
            LUAU_INSN_AUX_KV16(aux)
        } else {
            aux
        },
        cl,
        k,
    );
    luaur_common::LUAU_ASSERT!(ttisstring!(kv as *const TValue));

    if ttistable!(rb as *const TValue) {
        let h = hvalue!(rb as *const TValue);

        if fastnotm((*h).metatable, TMS::TM_NEWINDEX as i32) && (*h).readonly == 0 {
            VM_PROTECT_PC(L, pc_ptr);

            let res = luaH_setstr(L, h, tsvalue!(kv as *const TValue) as *mut _);
            VM_PATCH_C(pc_ptr.sub(2), gval2slot!(h, res as *const TValue));
            setobj2t!(L, res, ra as *const TValue);
            luaC_barriert!(L, h, ra as *const TValue);
            return pc_ptr;
        }

        let slot = (LUAU_INSN_C(insn) as i32) & (*h).nodemask8 as i32;
        (*L).cachedslot = slot;
        vm_protect!(L, pc_ptr, base, {
            lua_v_settable(L, rb as *const TValue, kv, ra);
        });
        VM_PATCH_C(pc_ptr.sub(2), (*L).cachedslot);
        return pc_ptr;
    }

    let mut fn_tm: *const TValue = core::ptr::null();
    if ttisuserdata!(rb as *const TValue)
        && {
            fn_tm = fasttm(
                L,
                (*uvalue!(rb as *const TValue)).metatable,
                TMS::TM_NEWINDEX as i32,
            );
            !fn_tm.is_null()
        }
        && ttisfunction!(fn_tm)
        && (*clvalue!(fn_tm)).isC != 0
    {
        luaur_common::LUAU_ASSERT!((*L).top.add(4) < (*L).stack.add((*L).stacksize as usize));
        let top = (*L).top;
        setobj2s!(L, top.add(0), fn_tm);
        setobj2s!(L, top.add(1), rb as *const TValue);
        setobj2s!(L, top.add(2), kv as *const TValue);
        setobj2s!(L, top.add(3), ra as *const TValue);
        (*L).top = top.add(4);

        (*L).cachedslot = LUAU_INSN_C(insn) as i32;
        vm_protect!(L, pc_ptr, base, {
            lua_v_call_tm(L, 3, -1);
        });
        VM_PATCH_C(pc_ptr.sub(2), (*L).cachedslot);
        return pc_ptr;
    }

    vm_protect!(L, pc_ptr, base, {
        lua_v_settable(L, rb as *const TValue, kv, ra);
    });
    pc_ptr
}

#[export_name = "luaur_executeSETTABLEKS"]
pub unsafe extern "C" fn executeSETTABLEKS(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_settableks(L, pc, base, k)
}
