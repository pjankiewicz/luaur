use luaur_vm::functions::lua_d_call::lua_d_call;
use luaur_vm::macros::setobj_2_s::setobj_2_s;
use luaur_vm::macros::ttisnil::ttisnil;
use luaur_vm::type_aliases::t_value::TValue;

use crate::macros::vm_reg::VM_REG;
use crate::type_aliases::lua_state::LuaState;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

pub unsafe fn forg_loop_non_table_fallback_deprecated(
    L: *mut luaur_vm::records::lua_state::lua_State,
    insn_a: i32,
    aux: i32,
) -> bool {
    let l_ptr = L;

    let base: *mut TValue = (*l_ptr).base;
    let mut ra: *mut TValue = VM_REG!(insn_a, l_ptr, base);

    // note: it's safe to push arguments past top for complicated reasons (see lvmexecute.cpp)
    setobj_2_s!(l_ptr, ra.add(3 + 2), ra.add(2));
    setobj_2_s!(l_ptr, ra.add(3 + 1), ra.add(1));
    setobj_2_s!(l_ptr, ra.add(3), ra);

    (*l_ptr).top = ra.add(3 + 3); // func + 2 args (state and index)
    LUAU_ASSERT!((*l_ptr).top <= (*l_ptr).stack_last);

    lua_d_call(l_ptr, ra.add(3), aux as u8 as i32);
    (*l_ptr).top = (*(*l_ptr).ci).top;

    // recompute ra since stack might have been reallocated
    let base = (*l_ptr).base;
    ra = VM_REG!(insn_a, l_ptr, base);

    // copy first variable back into the iteration index
    setobj_2_s!(l_ptr, ra.add(2), ra.add(3));

    !ttisnil!(ra.add(3))
}

#[export_name = "luaur_forgLoopNonTableFallback_DEPRECATED"]
pub unsafe extern "C" fn forgLoopNonTableFallback_DEPRECATED(
    L: *mut luaur_vm::records::lua_state::LuaState,
    insn_a: i32,
    aux: i32,
) -> bool {
    forg_loop_non_table_fallback_deprecated(L, insn_a, aux)
}
