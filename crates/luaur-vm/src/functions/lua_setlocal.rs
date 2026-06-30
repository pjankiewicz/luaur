use crate::functions::currentpc::currentpc;
use crate::functions::getluaproto::get_lua_proto;
use crate::functions::lua_f_getlocal::luaF_getlocal;
use crate::macros::api_check::api_check;
use crate::macros::getstr::getstr;
use crate::macros::lua_callinfo_native::LUA_CALLINFO_NATIVE;
use crate::macros::setobj_2_s::setobj_2_s;
use crate::records::call_info::CallInfo;
use crate::records::loc_var::LocVar;
use crate::records::lua_state::lua_State;
use crate::records::proto::Proto;

#[export_name = "luaur_lua_setlocal"]
pub unsafe fn lua_setlocal(
    L: *mut lua_State,
    level: core::ffi::c_int,
    n: core::ffi::c_int,
) -> *const core::ffi::c_char {
    api_check!(L, (*L).top.offset_from((*L).base) >= 1);

    if (level as u32) >= ((*L).ci.offset_from((*L).base_ci) as u32) {
        return core::ptr::null();
    }

    let ci: *mut CallInfo = (*L).ci.offset(-(level as isize));

    // changing registers in native functions externally may invalidate safety contracts wrt register type tags
    if ((*ci).flags & LUA_CALLINFO_NATIVE as u32) != 0 {
        return core::ptr::null();
    }

    let fp: *mut Proto = get_lua_proto(ci);
    let var: *const LocVar = if !fp.is_null() {
        luaF_getlocal(fp, n, currentpc(L, ci))
    } else {
        core::ptr::null()
    };

    if !var.is_null() {
        setobj_2_s!(
            L,
            (*ci).base.offset((*var).reg as isize),
            (*L).top.offset(-1)
        );
    }

    (*L).top = (*L).top.offset(-1); // pop value

    if !var.is_null() {
        getstr((*var).varname)
    } else {
        core::ptr::null()
    }
}
