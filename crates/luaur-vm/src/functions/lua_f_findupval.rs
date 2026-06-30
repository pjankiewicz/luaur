use crate::enums::lua_type::lua_Type;
use crate::macros::isblack::isblack;
use crate::macros::isdead::isdead;
use crate::macros::lua_c_init::luaC_init;
use crate::macros::upisopen::upisopen;
use crate::records::gc_object::GCObject;
use crate::records::up_val::UpVal;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use core::ffi::c_int;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn luaF_findupval(l: *mut lua_State, level: StkId) -> *mut UpVal {
    let g = (*l).global;
    let mut pp: *mut *mut UpVal = core::ptr::addr_of_mut!((*l).openupval);

    while !(*pp).is_null() && (*(*pp)).v >= level {
        let p = *pp;
        LUAU_ASSERT!(!isdead!(g, p as *mut GCObject));
        LUAU_ASSERT!(upisopen!(p));
        if (*p).v == level {
            return p;
        }

        pp = core::ptr::addr_of_mut!((*p).u.open.threadnext);
    }

    LUAU_ASSERT!((*l).isactive);
    LUAU_ASSERT!(!isblack!(l as *mut GCObject));

    let uv = crate::functions::lua_m_newgco::luaM_newgco_(
        l,
        core::mem::size_of::<UpVal>(),
        (*l).activememcat,
    ) as *mut UpVal;

    luaC_init!(l, uv, lua_Type::LUA_TUPVAL as c_int);
    (*uv).markedopen = 0;
    (*uv).v = level;

    (*uv).u.open.threadnext = *pp;
    *pp = uv;

    let uvhead = core::ptr::addr_of_mut!((*g).uvhead);
    (*uv).u.open.prev = uvhead;
    (*uv).u.open.next = (*g).uvhead.u.open.next;
    (*(*uv).u.open.next).u.open.prev = uv;
    (*g).uvhead.u.open.next = uv;

    LUAU_ASSERT!((*(*uv).u.open.next).u.open.prev == uv && (*(*uv).u.open.prev).u.open.next == uv);

    uv
}

#[allow(unused_imports)]
pub use luaF_findupval as lua_f_findupval;

#[export_name = "luaur_luaF_findupval"]
pub unsafe extern "C" fn lua_f_findupval_export(
    l: *mut lua_State,
    level: StkId,
) -> *mut core::ffi::c_void {
    luaF_findupval(l, level).cast()
}
