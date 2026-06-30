use crate::functions::index_2_addr::index2addr;
use crate::functions::lua_v_lessthan::lua_v_lessthan;
use crate::macros::lua_o_nilobject::luaO_nilobject;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_lua_lessthan"]
#[allow(non_snake_case)]
pub unsafe fn lua_lessthan(
    L: *mut lua_State,
    index1: core::ffi::c_int,
    index2: core::ffi::c_int,
) -> core::ffi::c_int {
    let o1: StkId = index2addr(L, index1);
    let o2: StkId = index2addr(L, index2);

    let nil_ptr = luaO_nilobject as *const TValue;

    if (o1 as *const TValue) == nil_ptr || (o2 as *const TValue) == nil_ptr {
        0
    } else {
        lua_v_lessthan(L, o1 as *const TValue, o2 as *const TValue)
    }
}
