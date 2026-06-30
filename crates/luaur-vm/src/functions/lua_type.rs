use core::ffi::c_int;

use crate::functions::index_2_addr::index2addr;
use crate::macros::lua_o_nilobject::luaO_nilobject;
use crate::macros::lua_tnone::LUA_TNONE;
use crate::macros::ttype::ttype;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

#[export_name = "luaur_lua_type"]
#[allow(non_snake_case)]
pub unsafe fn lua_type(L: *mut lua_State, idx: c_int) -> c_int {
    let o: StkId = index2addr(L, idx);

    if o == luaO_nilobject as StkId {
        LUA_TNONE
    } else {
        ttype!(o) as c_int
    }
}
