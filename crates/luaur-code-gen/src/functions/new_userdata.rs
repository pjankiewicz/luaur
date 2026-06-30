use core::ffi::c_int;

use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_vm::macros::isblack::isblack;
use luaur_vm::records::gc_object::GCObject;
use luaur_vm::records::lua_state::lua_State;
use luaur_vm::records::udata::Udata;

unsafe extern "C" {
    #[link_name = "luaU_newudata"]
    #[link_name = "luaur_luaU_newudata"]
    fn luaU_newudata(L: *mut lua_State, s: usize, tag: c_int) -> *mut Udata;
}

#[allow(non_snake_case)]
pub unsafe fn new_userdata(L: *mut lua_State, s: usize, tag: i32) -> *mut Udata {
    let u = luaU_newudata(L, s, tag);

    let h = (*(*L).global).udatamt[tag as usize];
    if !h.is_null() {
        // currently, we always allocate unmarked objects, so forward barrier can be skipped
        LUAU_ASSERT!(!isblack!(u as *mut GCObject));

        (*u).metatable = h;
    }

    u
}

#[export_name = "luaur_newUserdata"]
pub unsafe extern "C" fn newUserdata(
    L: *mut lua_State,
    s: usize,
    tag: c_int,
) -> *mut core::ffi::c_void {
    new_userdata(L, s, tag).cast()
}
