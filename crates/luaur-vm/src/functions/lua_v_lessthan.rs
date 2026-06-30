use core::ffi::c_int;

use crate::functions::call_order_tm::call_orderTM;
use crate::functions::lua_g_ordererror::luaG_ordererror;
use crate::functions::lua_v_strcmp::luaV_strcmp;
use crate::macros::luai_numlt::luai_numlt;
use crate::macros::nvalue::nvalue;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttisstring::ttisstring;
use crate::macros::ttype::ttype;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;
use luaur_common::macros::luau_likely::LUAU_LIKELY;
use luaur_common::macros::luau_unlikely::LUAU_UNLIKELY;

#[allow(non_snake_case)]
pub unsafe fn lua_v_lessthan(L: *mut lua_State, l: *const TValue, r: *const TValue) -> c_int {
    if LUAU_UNLIKELY!(ttype!(l) != ttype!(r)) {
        luaG_ordererror(L, l, r, TMS::TM_LT);
    } else if LUAU_LIKELY!(ttisnumber!(l)) {
        luai_numlt(nvalue!(l), nvalue!(r)) as c_int
    } else if ttisstring!(l) {
        if luaV_strcmp(tsvalue!(l), tsvalue!(r)) < 0 {
            1
        } else {
            0
        }
    } else {
        call_orderTM(L, l, r, TMS::TM_LT, true)
    }
}

#[export_name = "luaur_luaV_lessthan"]
pub unsafe extern "C" fn lua_v_lessthan_export(
    L: *mut lua_State,
    l: *const TValue,
    r: *const TValue,
) -> c_int {
    lua_v_lessthan(L, l, r)
}
