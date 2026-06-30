use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_l_optlstring::lua_l_optlstring;
use crate::functions::lua_toboolean::lua_toboolean;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::CStr;

#[export_name = "luaur_lua_b_assert"]
pub unsafe fn lua_b_assert(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checkany(L, 1);
    if lua_toboolean(L, 1) == 0 {
        let mut len = 0;
        let msg = lua_l_optlstring(L, 2, c"assertion failed!".as_ptr(), &mut len);
        let msg = CStr::from_ptr(msg).to_string_lossy();
        lua_l_error_l(L, c"%s".as_ptr(), format_args!("{}", msg));
    }
    lua_gettop(L)
}
