//! Node: `cxx:Function:Luau.CodeGen:CodeGen/src/CodeGenUtils.cpp:762:execute_forgprep`
//! Source: `CodeGen/src/CodeGenUtils.cpp`
//! Graph edges:
//! - declared_by: source_file CodeGen/src/CodeGenUtils.cpp
//! - source_includes:
//!   - includes -> source_file CodeGen/src/CodeGenUtils.h
//!   - includes -> source_file VM/src/lvm.h
//!   - includes -> source_file VM/src/lbuiltins.h
//!   - includes -> source_file VM/src/lbytecode.h
//!   - includes -> source_file VM/src/ldebug.h
//!   - includes -> source_file VM/src/ldo.h
//!   - includes -> source_file VM/src/lfunc.h
//!   - includes -> source_file VM/src/lgc.h
//!   - includes -> source_file VM/src/lmem.h
//!   - includes -> source_file VM/src/lnumutils.h
//!   - includes -> source_file VM/src/lstate.h
//!   - includes -> source_file VM/src/lstring.h
//!   - includes -> source_file VM/src/ltable.h
//!   - includes -> source_file VM/src/ludata.h
//! - incoming:
//!   - declares <- source_file CodeGen/src/CodeGenUtils.cpp
//! - outgoing:
//!   - type_ref -> type_alias lua_State (CodeGen/include/luacodegen.h)
//!   - type_ref -> type_alias StkId (VM/src/lobject.h)
//!   - type_ref -> type_alias TValue (VM/src/lobject.h)
//!   - calls -> macro clvalue (VM/src/lobject.h)
//!   - calls -> macro VM_REG (CodeGen/src/CodeGenUtils.cpp)
//!   - reads_global -> macro VM_REG (CodeGen/src/CodeGenUtils.cpp)
//!   - calls -> macro LUAU_INSN_A (Common/include/Luau/Bytecode.h)
//!   - reads_global -> macro LUAU_INSN_A (Common/include/Luau/Bytecode.h)
//!   - calls -> macro ttisfunction (VM/src/lobject.h)
//!   - calls -> macro ttistable (VM/src/lobject.h)
//!   - calls -> macro hvalue (VM/src/lobject.h)
//!   - calls -> macro ttisuserdata (VM/src/lobject.h)
//!   - calls -> macro uvalue (VM/src/lobject.h)
//!   - calls -> macro cast_to (VM/src/lcommon.h)
//!   - calls -> macro fasttm (VM/src/ltm.h)
//!   - calls -> macro setobj2s (VM/src/lobject.h)
//!   - calls -> macro LUAU_ASSERT (Common/include/Luau/Common.h)
//!   - reads_global -> macro LUAU_ASSERT (Common/include/Luau/Common.h)
//!   - calls -> macro VM_PROTECT (CodeGen/src/CodeGenUtils.cpp)
//!   - reads_global -> macro VM_PROTECT (CodeGen/src/CodeGenUtils.cpp)
//!   - calls -> function luaD_call (VM/src/ldo.cpp)
//!   - calls -> macro ttisnil (VM/src/lobject.h)
//!   - calls -> macro VM_PROTECT_PC (CodeGen/src/CodeGenUtils.cpp)
//!   - reads_global -> macro VM_PROTECT_PC (CodeGen/src/CodeGenUtils.cpp)
//!   - calls -> macro luaG_typeerror (VM/src/ldebug.h)
//!   - calls -> macro setpvalue (VM/src/lobject.h)
//!   - reads_global -> macro LU_TAG_ITERATOR (VM/src/lobject.h)
//!   - calls -> macro setnilvalue (VM/src/lobject.h)
//!   - calls -> macro LUAU_INSN_D (Common/include/Luau/Bytecode.h)
//!   - reads_global -> macro LUAU_INSN_D (Common/include/Luau/Bytecode.h)
//!   - translates_to -> rust_item executeFORGPREP

use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::fasttm::fasttm;
use luaur_vm::macros::hvalue::hvalue;
use luaur_vm::macros::lu_tag_iterator::LU_TAG_ITERATOR;
use luaur_vm::macros::setnilvalue::setnilvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::macros::setpvalue::setpvalue;
use luaur_vm::macros::ttisfunction::ttisfunction;
use luaur_vm::macros::ttisnil::ttisnil;
use luaur_vm::macros::ttistable::ttistable;
use luaur_vm::macros::ttisuserdata::ttisuserdata;
use luaur_vm::macros::uvalue::uvalue;
use luaur_vm::records::lua_table::LuaTable;
use luaur_vm::type_aliases::stk_id::StkId;
use luaur_vm::type_aliases::t_value::TValue;
use luaur_vm::type_aliases::tms::TMS;

use crate::macros::vm_protect::vm_protect;
use crate::macros::vm_protect_pc::VM_PROTECT_PC;
use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;

use luaur_common::macros::luau_insn_a::LUAU_INSN_A;
use luaur_common::macros::luau_insn_d::LUAU_INSN_D;

pub unsafe fn execute_forgprep(
    L: *mut lua_State,
    pc: *const Instruction,
    mut base: StkId,
    _k: *mut TValue,
) -> *const Instruction {
    let _cl = clvalue!((*(*L).ci).func);

    let mut pc = pc;
    let insn = *pc;
    pc = pc.add(1);

    let mut ra = VM_REG!(LUAU_INSN_A!(insn) as i32, L, base) as *mut TValue;

    if ttisfunction!(ra) {
        // will be called during FORGLOOP
    } else {
        let mt: *mut LuaTable = if ttistable!(ra) {
            (*hvalue!(ra)).metatable
        } else if ttisuserdata!(ra) {
            (*uvalue!(ra as *const TValue)).metatable
        } else {
            core::ptr::null_mut()
        };

        let fn_iter = fasttm(L, mt, TMS::TM_ITER as i32);

        if !fn_iter.is_null() {
            setobj2s!(L, ra.add(1), ra);
            setobj2s!(L, ra, fn_iter);

            (*L).top = ra.add(2); // func + self arg

            vm_protect!(L, pc, base, lua_d_call_protected(L, ra));
            (*L).top = (*(*L).ci).top;

            // recompute ra since stack might have been reallocated
            ra = VM_REG!(LUAU_INSN_A!(insn) as i32, L, base) as *mut TValue;

            // protect against __iter returning nil
            if ttisnil!(ra) {
                VM_PROTECT_PC(L, pc as *const u32);
                luaur_vm::functions::lua_g_typeerror_l::lua_g_typeerror_l(
                    L,
                    ra as *const TValue,
                    c"call".as_ptr(),
                );
            }
        } else if !fasttm(L, mt, TMS::TM_CALL as i32).is_null() {
            // table or userdata with __call, will be called during FORGLOOP
        } else if ttistable!(ra) {
            // set up registers for builtin iteration
            setobj2s!(L, ra.add(1), ra);
            setpvalue!(ra.add(2), core::ptr::null_mut(), LU_TAG_ITERATOR);
            setnilvalue!(ra);
        } else {
            VM_PROTECT_PC(L, pc as *const u32);
            luaur_vm::functions::lua_g_typeerror_l::lua_g_typeerror_l(
                L,
                ra as *const TValue,
                c"iterate over".as_ptr(),
            );
        }
    }

    pc = pc.offset(LUAU_INSN_D!(insn) as isize);
    pc
}

#[inline]
unsafe fn lua_d_call_protected(L: *mut lua_State, ra: StkId) {
    luaur_vm::functions::lua_d_call::lua_d_call(L, ra, 3);
}

#[export_name = "luaur_executeFORGPREP"]
pub unsafe extern "C" fn executeFORGPREP(
    L: *mut lua_State,
    pc: *const Instruction,
    base: StkId,
    k: *mut TValue,
) -> *const Instruction {
    execute_forgprep(L, pc, base, k)
}
