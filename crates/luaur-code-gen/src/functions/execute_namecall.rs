use crate::macros::vm_patch_c::VM_PATCH_C;
use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_aux_kv_16::LUAU_INSN_AUX_KV16;
use luaur_common::macros::luau_insn_b::LUAU_INSN_B;
use luaur_common::macros::luau_insn_c::LUAU_INSN_C;
use luaur_common::macros::luau_insn_op::LUAU_INSN_OP;
use luaur_vm::functions::lua_g_methoderror::luaG_methoderror;
use luaur_vm::functions::lua_v_gettable::lua_v_gettable;
use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::fasttm::fasttm;
use luaur_vm::macros::gkey::{gkey, gval};
use luaur_vm::macros::hvalue::hvalue;
use luaur_vm::macros::setobj_2_s::setobj_2_s as setobj2s;
use luaur_vm::macros::tsvalue::tsvalue;
use luaur_vm::macros::ttisnil::ttisnil;
use luaur_vm::macros::ttisstring::ttisstring;
use luaur_vm::macros::ttistable::ttistable;
use luaur_vm::macros::ttisuserdata::ttisuserdata;
use luaur_vm::macros::ttype::ttype;
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

pub unsafe fn execute_namecall(
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
    let mut ra = VM_REG!(LUAU_INSN_A(insn) as i32, L, base) as *mut TValue;
    let rb = VM_REG!(LUAU_INSN_B(insn) as i32, L, base) as *mut TValue;
    let aux = *pc_ptr;
    pc_ptr = pc_ptr.add(1);
    let kv = vm_kv(
        if op == LuauOpcode::LOP_NAMECALLUDATA as u32 {
            LUAU_INSN_AUX_KV16(aux)
        } else {
            aux
        },
        cl,
        k,
    );
    luaur_common::LUAU_ASSERT!(ttisstring!(kv as *const TValue));

    if ttistable!(rb as *const TValue) {
        setobj2s!(L, ra.add(1), rb as *const TValue);
        (*L).cachedslot = LUAU_INSN_C(insn) as i32;
        vm_protect!(L, pc_ptr, base, {
            lua_v_gettable(L, rb as *const TValue, kv, ra);
        });
        VM_PATCH_C(pc_ptr.sub(2), (*L).cachedslot);

        ra = VM_REG!(LUAU_INSN_A(insn) as i32, L, base) as *mut TValue;
        if ttisnil!(ra as *const TValue) {
            luaG_methoderror(L, ra.add(1) as *const TValue, tsvalue!(kv as *const TValue));
        }
    } else {
        let mt = if ttisuserdata!(rb as *const TValue) {
            (*uvalue!(rb as *const TValue)).metatable
        } else {
            (*(*L).global).mt[ttype!(rb as *const TValue) as usize]
        };

        let fn_nc = fasttm(L, mt, TMS::TM_NAMECALL as i32);
        if !fn_nc.is_null() {
            setobj2s!(L, ra.add(1), rb as *const TValue);
            setobj2s!(L, ra, fn_nc);

            (*L).namecall = tsvalue!(kv as *const TValue) as *mut _;
        } else {
            let tmi = fasttm(L, mt, TMS::TM_INDEX as i32);
            if !tmi.is_null() && ttistable!(tmi) {
                let h = hvalue!(tmi);
                let slot = (LUAU_INSN_C(insn) as i32) & (*h).nodemask8 as i32;
                let n = (*h).node.add(slot as usize);

                if ttisstring!(gkey!(n) as *const TValue)
                    && tsvalue!(gkey!(n) as *const TValue) == tsvalue!(kv as *const TValue)
                    && !ttisnil!(gval!(n))
                {
                    setobj2s!(L, ra.add(1), rb as *const TValue);
                    setobj2s!(L, ra, gval!(n));
                } else {
                    setobj2s!(L, ra.add(1), rb as *const TValue);
                    (*L).cachedslot = slot;
                    vm_protect!(L, pc_ptr, base, {
                        lua_v_gettable(L, rb as *const TValue, kv, ra);
                    });
                    VM_PATCH_C(pc_ptr.sub(2), (*L).cachedslot);

                    ra = VM_REG!(LUAU_INSN_A(insn) as i32, L, base) as *mut TValue;
                    if ttisnil!(ra as *const TValue) {
                        luaG_methoderror(
                            L,
                            ra.add(1) as *const TValue,
                            tsvalue!(kv as *const TValue),
                        );
                    }
                }
            } else {
                setobj2s!(L, ra.add(1), rb as *const TValue);
                vm_protect!(L, pc_ptr, base, {
                    lua_v_gettable(L, rb as *const TValue, kv, ra);
                });

                ra = VM_REG!(LUAU_INSN_A(insn) as i32, L, base) as *mut TValue;
                if ttisnil!(ra as *const TValue) {
                    luaG_methoderror(L, ra.add(1) as *const TValue, tsvalue!(kv as *const TValue));
                }
            }
        }
    }

    luaur_common::LUAU_ASSERT!(
        LUAU_INSN_OP(*pc_ptr) == LuauOpcode::LOP_CALL as u32
            || LUAU_INSN_OP(*pc_ptr) == LuauOpcode::LOP_CALLFB as u32
    );
    pc_ptr
}

#[export_name = "luaur_executeNAMECALL"]
pub unsafe extern "C" fn executeNAMECALL(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_namecall(L, pc, base, k)
}
