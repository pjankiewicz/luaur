use crate::functions::lua_createtable::lua_createtable;
use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_h_setstr::lua_h_setstr;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_s_newliteral::luaS_newliteral;
use crate::macros::setnvalue::setnvalue;
use crate::macros::setobj_2_t::setobj2t;
use crate::records::lua_state::lua_State;
use crate::records::lua_t_value::TValue;
use crate::records::lua_table::LuaTable;

#[export_name = "luaur_tpack"]
pub unsafe fn tpack(L: *mut lua_State) -> core::ffi::c_int {
    let n = lua_gettop(L); // number of elements to pack
    lua_createtable(L, n, 1); // create result table

    let t: *mut LuaTable = hvalue!((*L).top.offset(-1));

    for i in 0..n as usize {
        let e: *mut TValue = (*t).array.add(i);
        setobj2t!(L, e, (*L).base.add(i));
    }

    // t.n = number of elements
    let nv = lua_h_setstr(
        L,
        t,
        luaS_newliteral(L, b"n\0" as *const _ as *const core::ffi::c_char),
    );
    setnvalue!(nv, n as f64);

    1 // return table
}
