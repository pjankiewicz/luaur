use crate::functions::lua_h_getnum::lua_h_getnum;
use crate::functions::newkey::newkey;
use crate::macros::cast_num::cast_num;
use crate::macros::cast_to::cast_to;
use crate::macros::setnvalue::setnvalue;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

use crate::macros::lua_o_nilobject::luaO_nilobject;

#[allow(non_snake_case)]
pub unsafe fn luaH_setnum(
    l: *mut lua_State,
    t: *mut LuaTable,
    key: core::ffi::c_int,
) -> *mut TValue {
    // (1 <= key && key <= t->sizearray)
    if (key as core::ffi::c_uint).wrapping_sub(1) < (*t).sizearray as core::ffi::c_uint {
        return (*t).array.add((key - 1) as usize);
    }

    // hash fallback
    let p = lua_h_getnum(t, key);
    if p != luaO_nilobject {
        cast_to!(*mut TValue, p)
    } else {
        let mut k: TValue = core::mem::zeroed();
        setnvalue!(&mut k, cast_num!(key));

        // The skeleton for newkey is a stub with no arguments.
        // We must cast the call to match the expected signature (L, t, key) -> TValue*.
        core::mem::transmute::<
            _,
            unsafe fn(*mut lua_State, *mut LuaTable, *const TValue) -> *mut TValue,
        >(newkey as *const core::ffi::c_void)(l, t, &k)
    }
}

#[export_name = "luaur_luaH_setnum"]
pub unsafe extern "C" fn lua_h_setnum_export(
    l: *mut lua_State,
    t: *mut core::ffi::c_void,
    key: core::ffi::c_int,
) -> *mut TValue {
    luaH_setnum(l, t as *mut LuaTable, key)
}
