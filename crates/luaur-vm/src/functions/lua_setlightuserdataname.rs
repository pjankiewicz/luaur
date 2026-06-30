use crate::macros::api_check::api_check;
use crate::macros::fixedbit::FIXEDBIT;
use crate::macros::l_setbit::l_setbit;
use crate::macros::lua_lutag_limit::LUA_LUTAG_LIMIT;
use crate::macros::lua_s_fix::luaS_fix;
use crate::macros::lua_s_new::luaS_new;
use crate::records::t_string::TString;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_setlightuserdataname"]
#[allow(non_snake_case)]
pub unsafe fn lua_setlightuserdataname(
    L: *mut lua_State,
    tag: core::ffi::c_int,
    name: *const core::ffi::c_char,
) {
    api_check!(L, (tag as u32) < LUA_LUTAG_LIMIT as u32);
    // renaming not supported
    api_check!(L, (*(*L).global).lightuserdataname[tag as usize].is_null());

    if (*(*L).global).lightuserdataname[tag as usize].is_null() {
        let ts = luaS_new(L, name);
        (*(*L).global).lightuserdataname[tag as usize] = ts;
        l_setbit!((*ts).hdr.marked, FIXEDBIT); // never collect these names
    }
}
