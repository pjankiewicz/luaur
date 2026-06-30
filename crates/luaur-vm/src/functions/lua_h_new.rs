use crate::enums::lua_type::lua_Type;
use crate::functions::setarrayvector::setarrayvector;
use crate::functions::setnodevector::setnodevector;
use crate::macros::dummynode::dummynode;
use crate::macros::lua_c_init::luaC_init;
use crate::records::gc_object::GCObject;
use crate::records::lua_node::LuaNode;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[allow(non_snake_case)]
pub unsafe fn lua_h_new(l: *mut lua_State, narray: c_int, nhash: c_int) -> *mut LuaTable {
    let t = crate::functions::lua_m_newgco::luaM_newgco_(
        l,
        core::mem::size_of::<LuaTable>(),
        (*l).activememcat,
    ) as *mut LuaTable;

    luaC_init!(l, t, lua_Type::LUA_TTABLE as c_int);
    (*t).metatable = core::ptr::null_mut();
    (*t).tmcache = !0u8;
    (*t).array = core::ptr::null_mut();
    (*t).sizearray = 0;
    (*t).union.lastfree = 0;
    (*t).lsizenode = 0;
    (*t).readonly = 0;
    (*t).safeenv = 0;
    (*t).nodemask8 = 0;
    (*t).node = dummynode as *mut LuaNode;

    if narray > 0 {
        setarrayvector(l, t, narray);
    }

    if nhash > 0 {
        setnodevector(l, t, nhash);
    }

    t
}

#[export_name = "luaur_luaH_new"]
pub unsafe extern "C" fn lua_h_new_export(
    l: *mut lua_State,
    narray: c_int,
    nhash: c_int,
) -> *mut core::ffi::c_void {
    lua_h_new(l, narray, nhash).cast()
}

#[allow(unused_imports)]
pub use lua_h_new as luaH_new;
