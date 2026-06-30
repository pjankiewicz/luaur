use crate::functions::lua_createtable::lua_createtable;
use crate::functions::lua_l_checkinteger::lua_l_checkinteger;
use crate::luaL_argerror;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_isnoneornil::lua_isnoneornil;
use crate::setobj2t;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_tcreate"]
pub unsafe fn tcreate(L: *mut lua_State) -> core::ffi::c_int {
    let size = lua_l_checkinteger(L, 1);
    if size < 0 {
        luaL_argerror!(L, 1, "size out of range");
    }

    if !lua_isnoneornil!(L, 2) {
        lua_createtable(L, size as core::ffi::c_int, 0);
        let t = hvalue!((*L).top.offset(-1));

        let v: StkId = (*L).base.add(1);

        for i in 0..size as usize {
            let e: *mut TValue = (*t).array.add(i);
            setobj2t!(L, e, v);
        }
    } else {
        lua_createtable(L, size as core::ffi::c_int, 0);
    }

    1
}
