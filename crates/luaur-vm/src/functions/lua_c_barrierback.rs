use crate::records::gc_object::GCObject;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_c_barrierback(l: *mut lua_State, o: *mut GCObject, gclist: *mut *mut GCObject) {
    let g = (*l).global;

    // isblack(o) is ((*o).gch.marked & 4) != 0
    // isdead(g, o) is ((*o).gch.marked & 11) == (((*g).currentwhite ^ 3) & 3)
    let is_black = ((*o).gch.marked & 4) != 0;
    let is_dead = ((*o).gch.marked & 11) == (((*g).currentwhite ^ 3) & 3);

    LUAU_ASSERT!(is_black && !is_dead);
    LUAU_ASSERT!((*g).gcstate as i32 != 0); // GCSpause is 0

    // black2gray(o) clears the BLACKBIT (bit 2, mask 4)
    (*o).gch.marked &= !4;

    *gclist = (*g).grayagain;
    (*g).grayagain = o;
}

#[export_name = "luaur_luaC_barrierback"]
pub unsafe extern "C" fn lua_c_barrierback_export(
    l: *mut lua_State,
    o: *mut core::ffi::c_void,
    gclist: *mut *mut core::ffi::c_void,
) {
    lua_c_barrierback(l, o as *mut GCObject, gclist as *mut *mut GCObject);
}
