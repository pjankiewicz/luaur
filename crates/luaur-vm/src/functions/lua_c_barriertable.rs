//! Node: `cxx:Function:Luau.VM:VM/src/lgc.cpp:1296:lua_c_barriertable`
//! Source: `VM/src/lgc.cpp` (lgc.cpp:1296-1314, hand-ported)

use crate::functions::reallymarkobject::reallymarkobject;
use crate::macros::black_2_gray::black2gray;
use crate::macros::gc_spause::GCSpause;
use crate::macros::gc_spropagateagain::GCSpropagateagain;
use crate::macros::isblack::isblack;
use crate::macros::isdead::isdead;
use crate::macros::iswhite::iswhite;
use crate::records::gc_object::GCObject;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn luaC_barriertable(l: *mut lua_State, t: *mut LuaTable, v: *mut GCObject) {
    let g = (*l).global;
    let o = t as *mut GCObject;

    // in the second propagation stage, table assignment barrier works as a forward barrier
    if (*g).gcstate as i32 == GCSpropagateagain {
        LUAU_ASSERT!(isblack!(o) && iswhite!(v) && !isdead!(g, v) && !isdead!(g, o));
        reallymarkobject(g, v);
        return;
    }

    LUAU_ASSERT!(isblack!(o) && !isdead!(g, o));
    LUAU_ASSERT!((*g).gcstate as i32 != GCSpause);
    black2gray!(o); // make table gray (again)
    (*t).gclist = (*g).grayagain;
    (*g).grayagain = o;
}

#[allow(unused_imports)]
pub use luaC_barriertable as lua_c_barriertable;

#[export_name = "luaur_luaC_barriertable"]
pub unsafe extern "C" fn lua_c_barriertable_export(
    l: *mut lua_State,
    t: *mut core::ffi::c_void,
    v: *mut core::ffi::c_void,
) {
    luaC_barriertable(l, t as *mut LuaTable, v as *mut GCObject);
}
