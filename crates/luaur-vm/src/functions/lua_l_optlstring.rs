use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_type::lua_type;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_char;
use core::ffi::c_int;

#[export_name = "luaur_lua_l_optlstring"]
pub unsafe fn lua_l_optlstring(
    L: *mut lua_State,
    narg: c_int,
    def: *const c_char,
    len: *mut usize,
) -> *const c_char {
    let is_none_or_nil = lua_type(L, narg) <= (crate::enums::lua_type::lua_Type::LUA_TNIL as c_int);

    if is_none_or_nil {
        if !len.is_null() {
            if !def.is_null() {
                let mut strlen: usize = 0;
                let mut p = def;
                while *p != 0 {
                    strlen += 1;
                    p = p.add(1);
                }
                *len = strlen;
            } else {
                *len = 0;
            }
        }
        def
    } else {
        lua_l_checklstring(L, narg, len)
    }
}
