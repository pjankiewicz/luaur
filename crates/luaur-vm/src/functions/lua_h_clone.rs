use crate::enums::lua_type::lua_Type;
use crate::macros::dummynode::dummynode;
use crate::macros::getaboundary::getaboundary;
use crate::macros::lua_c_init::luaC_init;
use crate::macros::lua_m_newarray::luaM_newarray;
use crate::records::lua_node::LuaNode;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;
use core::ffi::c_int;

#[inline]
unsafe fn maybesetaboundary(t: *mut LuaTable, boundary: c_int) {
    if (*t).union.aboundary <= 0 {
        (*t).union.aboundary = -boundary;
    }
}

#[allow(non_snake_case)]
pub unsafe fn lua_h_clone(l: *mut lua_State, tt: *mut LuaTable) -> *mut LuaTable {
    let t = crate::functions::lua_m_newgco::luaM_newgco_(
        l,
        core::mem::size_of::<LuaTable>(),
        (*l).activememcat,
    ) as *mut LuaTable;

    luaC_init!(l, t, lua_Type::LUA_TTABLE as c_int);
    (*t).metatable = (*tt).metatable;
    (*t).tmcache = (*tt).tmcache;
    (*t).array = core::ptr::null_mut();
    (*t).sizearray = 0;
    (*t).lsizenode = 0;
    (*t).nodemask8 = 0;
    (*t).readonly = 0;
    (*t).safeenv = 0;
    (*t).node = dummynode as *mut LuaNode;
    (*t).union.lastfree = 0;

    if (*tt).sizearray != 0 {
        (*t).array = luaM_newarray!(l, (*tt).sizearray as usize, TValue, (*t).memcat);
        maybesetaboundary(t, getaboundary(tt));
        (*t).sizearray = (*tt).sizearray;

        core::ptr::copy_nonoverlapping((*tt).array, (*t).array, (*t).sizearray as usize);
    }

    if (*tt).node != dummynode as *mut LuaNode {
        let size = 1i32 << (*tt).lsizenode;
        (*t).node = luaM_newarray!(l, size as usize, LuaNode, (*t).memcat);
        (*t).lsizenode = (*tt).lsizenode;
        (*t).nodemask8 = (*tt).nodemask8;
        core::ptr::copy_nonoverlapping((*tt).node, (*t).node, size as usize);
        (*t).union.lastfree = (*tt).union.lastfree;
    }

    t
}

#[allow(unused_imports)]
pub use lua_h_clone as luaH_clone;

#[export_name = "luaur_luaH_clone"]
pub unsafe extern "C" fn lua_h_clone_export(
    l: *mut lua_State,
    tt: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void {
    lua_h_clone(l, tt as *mut LuaTable).cast()
}
