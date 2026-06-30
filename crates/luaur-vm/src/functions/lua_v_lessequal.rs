use crate::enums::tms::TMS;
use crate::functions::call_order_tm::call_orderTM;
use crate::functions::lua_g_ordererror::luaG_ordererror;
use crate::functions::lua_v_strcmp::luaV_strcmp;
use crate::macros::luai_numle::luai_numle;
use crate::macros::nvalue::nvalue;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttisstring::ttisstring;
use crate::macros::ttype::ttype;
use crate::records::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[allow(non_snake_case)]
pub unsafe fn lua_v_lessequal(L: *mut lua_State, l: *const TValue, r: *const TValue) -> i32 {
    let mut res: i32 = 0;

    if ttype!(l) != ttype!(r) {
        luaG_ordererror(L, l, r, TMS::TM_LE);
    } else if ttisnumber!(l) {
        return luai_numle(nvalue!(l), nvalue!(r)) as i32;
    } else if ttisstring!(l) {
        return (luaV_strcmp(tsvalue!(l), tsvalue!(r)) <= 0) as i32;
    } else if {
        res = call_orderTM(L, l, r, TMS::TM_LE, false);
        res != -1
    } {
        // first try `le'
        return res;
    } else if {
        res = call_orderTM(L, r, l, TMS::TM_LT, false);
        res == -1
    } {
        // error if not `lt'
        luaG_ordererror(L, l, r, TMS::TM_LE);
    }

    (res == 0) as i32
}

#[export_name = "luaur_luaV_lessequal"]
pub unsafe extern "C" fn lua_v_lessequal_export(
    L: *mut lua_State,
    l: *const TValue,
    r: *const TValue,
) -> i32 {
    lua_v_lessequal(L, l, r)
}
