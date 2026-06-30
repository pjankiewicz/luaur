//! `luaG_onbreak` — returns true when the current instruction at the top
//! call frame is `LOP_BREAK`.
//! C++ source: `VM/src/ldebug.cpp:405`

use crate::enums::lua_type::lua_Type;
use crate::records::lua_state::lua_State;
use luaur_common::enums::luau_opcode::LuauOpcode;
use luaur_common::macros::luau_insn_op::LUAU_INSN_OP;

#[export_name = "luaur_luaG_onbreak"]
#[allow(non_snake_case)]
pub unsafe fn luaG_onbreak(l: *mut lua_State) -> bool {
    if (*l).ci == (*l).base_ci {
        return false;
    }

    // isLua(ci): ci->func is a function && !clvalue(ci->func)->isC
    // Inline to avoid the broken ttisfunction! macro.
    let func = (*(*l).ci).func;
    if (*func).tt() != lua_Type::LUA_TFUNCTION as core::ffi::c_int {
        return false;
    }
    if (*(*(*func).value.gc).cl).isC != 0 {
        return false;
    }

    LUAU_INSN_OP(*(*(*l).ci).savedpc) == LuauOpcode::LOP_BREAK as u32
}
