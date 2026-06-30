//! `lua_setuserdatametatable` — pop a table from the stack and register it as
//! the metatable for userdata of type `tag`.
//! C++ source: `VM/src/lapi.cpp:1616`

use crate::enums::lua_type::lua_Type;
use crate::macros::lua_utag_limit::LUA_UTAG_LIMIT;
use crate::records::lua_state::lua_State;
use crate::records::lua_table::LuaTable;

#[export_name = "luaur_lua_setuserdatametatable"]
#[allow(non_snake_case)]
pub unsafe fn lua_setuserdatametatable(l: *mut lua_State, tag: core::ffi::c_int) {
    crate::api_checknelems!(l, 1);
    crate::api_check!(l, (tag as u32) < LUA_UTAG_LIMIT as u32);
    // reassignment not supported
    crate::api_check!(l, (*(*l).global).udatamt[tag as usize].is_null());

    let t = (*l).top.offset(-1);
    crate::api_check!(l, (*t).tt == lua_Type::LUA_TTABLE as core::ffi::c_int);

    // hvalue(top-1): the gc pointer refers to a GcObject; its `h` union arm is a LuaTable.
    let gco = (*t).value.gc;
    let h: *mut LuaTable = core::ptr::addr_of_mut!((*gco).h) as *mut LuaTable;
    (*(*l).global).udatamt[tag as usize] = h;

    (*l).top = (*l).top.offset(-1);
}
