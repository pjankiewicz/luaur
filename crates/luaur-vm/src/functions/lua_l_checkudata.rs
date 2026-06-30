use crate::functions::lua_getfield::lua_getfield;
use crate::functions::lua_getmetatable::lua_getmetatable;
use crate::functions::lua_l_typeerror_l::lua_l_typeerror_l;
use crate::functions::lua_rawequal::lua_rawequal;
use crate::functions::lua_touserdata::lua_touserdata;
use crate::macros::lua_pop::lua_pop;
use crate::macros::lua_registryindex::LUA_REGISTRYINDEX;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_int, c_void};

#[export_name = "luaur_lua_l_checkudata"]
pub unsafe fn lua_l_checkudata(L: *mut lua_State, ud: c_int, tname: &str) -> *mut c_void {
    let p = lua_touserdata(L, ud);
    if !p.is_null() {
        if lua_getmetatable(L, ud) != 0 {
            let c_tname = std::ffi::CString::new(tname).unwrap();
            lua_getfield(L, LUA_REGISTRYINDEX, c_tname.as_ptr());

            if lua_rawequal(L, -1, -2) != 0 {
                lua_pop(L, 2);
                return p;
            }

            lua_pop(L, 2); // remove both metatables if they didn't match
        }
    }

    // lua_l_typeerror_l is l_noret (returns !), so this call never returns.
    lua_l_typeerror_l(L, ud, tname);
}
