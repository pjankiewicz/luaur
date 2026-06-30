use crate::functions::lua_d_check_cstack::luaD_checkCstack;
use crate::functions::performcall::performcall;
use crate::macros::isyielded::isyielded;
use crate::macros::lua_c_check_gc::luaC_checkGC;
use crate::macros::lua_multret::LUA_MULTRET;
use crate::macros::luai_maxccalls::LUAI_MAXCCALLS;
use crate::macros::restorestack::restorestack;
use crate::macros::savestack::savestack;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[export_name = "luaur_lua_d_callny"]
pub unsafe fn lua_d_callny(L: *mut lua_State, func: StkId, nresults: core::ffi::c_int) {
    let l_ref = &mut *L;

    l_ref.nCcalls += 1;
    if l_ref.nCcalls >= LUAI_MAXCCALLS as u16 {
        luaD_checkCstack(L);
    }

    LUAU_ASSERT!(l_ref.nCcalls > l_ref.baseCcalls);

    let funcoffset = savestack!(L, func);

    performcall(L, func, nresults, false);

    LUAU_ASSERT!(!isyielded(L));

    if nresults != LUA_MULTRET {
        (*L).top = restorestack!(L, funcoffset).add(nresults as usize);
    }

    (*L).nCcalls -= 1;
    luaC_checkGC!(L);
}
