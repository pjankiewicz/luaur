use crate::functions::lua_call::lua_call;
use crate::functions::lua_l_getmetafield::lua_l_getmetafield;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::macros::abs_index::abs_index;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

#[export_name = "luaur_lua_l_callmeta"]
pub unsafe fn lua_l_callmeta(L: *mut lua_State, obj: c_int, event: *const c_char) -> c_int {
    let obj = abs_index(L, obj);
    if lua_l_getmetafield(L, obj, event) == 0 {
        return 0;
    }

    lua_pushvalue(L, obj);
    lua_call(L, 1, 1);
    1
}
