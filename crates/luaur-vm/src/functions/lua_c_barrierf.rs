//! Node: `cxx:Function:Luau.VM:VM/src/lgc.cpp:1284:lua_c_barrierf`
//! Source: `VM/src/lgc.cpp` (lgc.cpp:1284-1294, hand-ported)

use crate::functions::reallymarkobject::reallymarkobject;
use crate::macros::gc_spause::GCSpause;
use crate::macros::isblack::isblack;
use crate::macros::isdead::isdead;
use crate::macros::iswhite::iswhite;
use crate::macros::keepinvariant::keepinvariant;
use crate::macros::makewhite::makewhite;
use crate::records::gc_object::GCObject;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn luaC_barrierf(l: *mut lua_State, o: *mut GCObject, v: *mut GCObject) {
    let g = (*l).global;
    LUAU_ASSERT!(isblack!(o) && iswhite!(v) && !isdead!(g, v) && !isdead!(g, o));
    LUAU_ASSERT!((*g).gcstate as i32 != GCSpause);
    // must keep invariant?
    if keepinvariant(g) {
        reallymarkobject(g, v); // restore invariant
    } else {
        // don't mind
        makewhite!(g, o); // mark as white just to avoid other barriers
    }
}

#[allow(unused_imports)]
pub use luaC_barrierf as lua_c_barrierf;

#[export_name = "luaur_luaC_barrierf"]
pub unsafe extern "C" fn lua_c_barrierf_export(
    l: *mut lua_State,
    o: *mut core::ffi::c_void,
    v: *mut core::ffi::c_void,
) {
    luaC_barrierf(l, o as *mut GCObject, v as *mut GCObject);
}
