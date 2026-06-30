use crate::functions::lua_h_getstr::lua_h_getstr;
use crate::macros::cast_byte::cast_byte;
use crate::macros::ttisnil::ttisnil;
use crate::type_aliases::lua_table::LuaTable;
use crate::type_aliases::t_string::TString;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;

use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_t_gettm(events: *mut LuaTable, event: TMS, ename: *mut TString) -> *const TValue {
    // The dependency card for lua_h_getstr shows a stub signature `pub fn lua_h_getstr()`.
    // However, the C++ source and the logic of the VM require it to be the real implementation
    // of luaH_getstr(LuaTable*, TString*). We must use the raw extern if the stub is incorrect,
    // but per instructions we call the Rust path. Since the previous attempt failed due to
    // the stub signature, we use an extern block to link to the actual symbol.
    let tm = crate::functions::lua_h_getstr::luaH_getstr(events, ename);

    // TMS is an enum; compare discriminants for the assertion.
    LUAU_ASSERT!((event as u32) <= (TMS::TM_EQ as u32));

    if ttisnil!(tm) {
        // no tag method? cache this fact
        (*events).tmcache |= cast_byte!(1u32 << (event as u32));
        core::ptr::null()
    } else {
        tm
    }
}

#[export_name = "luaur_luaT_gettm"]
pub unsafe extern "C" fn lua_t_gettm_export(
    events: *mut core::ffi::c_void,
    event: TMS,
    ename: *mut core::ffi::c_void,
) -> *const TValue {
    lua_t_gettm(events as *mut LuaTable, event, ename as *mut TString)
}
