use crate::enums::lua_status::lua_Status;
use crate::macros::lua_c_check_gc::luaC_checkGC;
use crate::macros::lua_callinfo_opyield::LUA_CALLINFO_OPYIELD;
use crate::macros::luai_maxccalls::LUAI_MAXCCALLS;
use crate::macros::restoreci::restoreci;
use crate::macros::saveci::saveci;
use crate::records::call_info::CallInfo;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

#[export_name = "luaur_lua_d_performcally"]
pub unsafe fn lua_d_performcally(
    L: *mut lua_State,
    func: StkId,
    nresults: core::ffi::c_int,
) -> bool {
    use crate::macros::check_exp::check_exp;

    (*L).nCcalls = (*L).nCcalls.wrapping_add(1);
    if (*L).nCcalls >= LUAI_MAXCCALLS as u16 {
        crate::functions::lua_d_check_cstack::luaD_checkCstack(L);
    }

    (*L).baseCcalls = (*L).baseCcalls.wrapping_add(1);

    let cioffset = saveci!(L, (*L).ci);

    crate::functions::performcall::performcall(L, func, nresults, false);

    if (*L).status != lua_Status::LUA_OK as u8 {
        let caller = restoreci!(L, cioffset);
        (*caller).flags |= LUA_CALLINFO_OPYIELD as u32;
        return true;
    }

    (*L).baseCcalls = (*L).baseCcalls.wrapping_sub(1);
    (*L).nCcalls = (*L).nCcalls.wrapping_sub(1);
    luaC_checkGC!(L);
    false
}
