use crate::functions::lua_g_forerror_l::lua_g_forerror_l;
use crate::functions::lua_v_tonumber::lua_v_tonumber;
use crate::macros::ttisnumber::ttisnumber;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

#[export_name = "luaur_lua_v_prepare_forn"]
pub unsafe fn lua_v_prepare_forn(L: *mut lua_State, plimit: StkId, pstep: StkId, pinit: StkId) {
    if !ttisnumber!(pinit) && lua_v_tonumber(pinit, pinit).is_null() {
        lua_g_forerror_l(
            L,
            pinit,
            b"initial value\0".as_ptr() as *const core::ffi::c_char,
        );
    }
    if !ttisnumber!(plimit) && lua_v_tonumber(plimit, plimit).is_null() {
        lua_g_forerror_l(L, plimit, b"limit\0".as_ptr() as *const core::ffi::c_char);
    }
    if !ttisnumber!(pstep) && lua_v_tonumber(pstep, pstep).is_null() {
        lua_g_forerror_l(L, pstep, b"step\0".as_ptr() as *const core::ffi::c_char);
    }
}
