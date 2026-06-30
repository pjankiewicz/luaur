use crate::functions::lua_d_callny::lua_d_callny;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

#[allow(non_snake_case)]
#[export_name = "luaur_luaB_xpcallerr"]
pub unsafe fn luaB_xpcallerr(L: *mut lua_State, ud: *mut core::ffi::c_void) {
    let func: StkId = ud as StkId;
    lua_d_callny(L, func, 1);
}
