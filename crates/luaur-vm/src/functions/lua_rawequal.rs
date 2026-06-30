use crate::functions::index_2_addr::index2addr;
use crate::functions::lua_o_rawequal_obj::luaO_rawequalObj;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

use crate::macros::lua_o_nilobject::luaO_nilobject;

#[export_name = "luaur_lua_rawequal"]
#[allow(non_snake_case)]
pub unsafe fn lua_rawequal(
    L: *mut lua_State,
    index1: core::ffi::c_int,
    index2: core::ffi::c_int,
) -> core::ffi::c_int {
    let o1: StkId = index2addr(L, index1);
    let o2: StkId = index2addr(L, index2);

    if o1 == luaO_nilobject as StkId || o2 == luaO_nilobject as StkId {
        0
    } else {
        luaO_rawequalObj(
            o1 as *const crate::type_aliases::t_value::TValue,
            o2 as *const crate::type_aliases::t_value::TValue,
        )
    }
}
