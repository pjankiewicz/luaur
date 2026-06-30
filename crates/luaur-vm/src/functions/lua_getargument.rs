use crate::functions::getluaproto::get_lua_proto;
use crate::functions::lua_a_pushvalue::luaA_pushvalue;
use crate::functions::lua_c_barrierback::lua_c_barrierback;
use crate::macros::lua_callinfo_native::LUA_CALLINFO_NATIVE;
use crate::records::call_info::CallInfo;
use crate::records::lua_state::lua_State;
use crate::records::proto::Proto;

#[export_name = "luaur_lua_getargument"]
pub unsafe fn lua_getargument(
    l: *mut lua_State,
    level: core::ffi::c_int,
    n: core::ffi::c_int,
) -> core::ffi::c_int {
    if (level as u32) >= ((*l).ci.offset_from((*l).base_ci) as u32) {
        return 0;
    }

    let ci: *mut CallInfo = (*l).ci.offset(-(level as isize));

    // changing tables in native functions externally may invalidate safety contracts wrt table state (metatable/size/readonly)
    if ((*ci).flags & LUA_CALLINFO_NATIVE as u32) != 0 {
        return 0;
    }

    let fp: *mut Proto = get_lua_proto(ci);
    let mut res: core::ffi::c_int = 0;

    if !fp.is_null() && n > 0 {
        if (n as u32) <= (*fp).numparams as u32 {
            if ((*l).hdr.marked & 4) != 0 {
                lua_c_barrierback(l, l as *mut _, &mut (*l).gclist);
            }
            luaA_pushvalue(l, (*ci).base.offset((n - 1) as isize));
            res = 1;
        } else if (*fp).is_vararg != 0 && (n as isize) < (*ci).base.offset_from((*ci).func) {
            if ((*l).hdr.marked & 4) != 0 {
                lua_c_barrierback(l, l as *mut _, &mut (*l).gclist);
            }
            luaA_pushvalue(l, (*ci).func.offset(n as isize));
            res = 1;
        }
    }

    res
}
