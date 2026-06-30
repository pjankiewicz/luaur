use crate::functions::lua_f_closeupval::luaF_closeupval;
use crate::macros::isblack::isblack;
use crate::macros::isdead::isdead;
use crate::macros::upisopen::upisopen;
use crate::records::gc_object::GCObject;
use crate::records::up_val::UpVal;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn luaF_close(l: *mut lua_State, level: StkId) {
    let g = (*l).global;

    while !(*l).openupval.is_null() && (*(*l).openupval).v >= level {
        let uv: *mut UpVal = (*l).openupval;
        let o = uv as *mut GCObject;
        LUAU_ASSERT!(!isblack!(o) && upisopen!(uv));
        LUAU_ASSERT!(!isdead!(g, o));

        (*l).openupval = (*uv).u.open.threadnext;
        luaF_closeupval(l, uv, false);
    }
}

#[allow(unused_imports)]
pub use luaF_close as lua_f_close;

#[export_name = "luaur_luaF_close"]
pub unsafe extern "C" fn lua_f_close_export(l: *mut lua_State, level: StkId) {
    luaF_close(l, level);
}
