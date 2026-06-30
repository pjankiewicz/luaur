use crate::functions::index_2_addr::index2addr;
use crate::macros::equalobj::equalobj;
use crate::macros::lua_o_nilobject::luaO_nilobject;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_lua_equal"]
#[allow(non_snake_case)]
pub unsafe fn lua_equal(
    L: *mut lua_State,
    index1: core::ffi::c_int,
    index2: core::ffi::c_int,
) -> core::ffi::c_int {
    let o1: StkId = index2addr(L, index1);
    let o2: StkId = index2addr(L, index2);

    let nil_ptr = luaO_nilobject as *const TValue;

    let i = if (o1 as *const TValue) == nil_ptr || (o2 as *const TValue) == nil_ptr {
        0
    } else {
        if equalobj!(L, o1 as *const TValue, o2 as *const TValue) {
            1
        } else {
            0
        }
    };

    i as core::ffi::c_int
}
