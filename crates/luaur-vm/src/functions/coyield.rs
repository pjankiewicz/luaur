use crate::functions::lua_yield::lua_yield;
use crate::macros::cast_int::cast_int;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_coyield"]
pub unsafe fn coyield(L: *mut lua_State) -> core::ffi::c_int {
    let nres = cast_int!((*L).top.offset_from((*L).base));
    lua_yield(L, nres)
}
