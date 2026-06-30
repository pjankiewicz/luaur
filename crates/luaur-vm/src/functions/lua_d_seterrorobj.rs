use crate::enums::lua_status::lua_Status;
use crate::macros::lua_s_newliteral::luaS_newliteral;
use crate::macros::setobj_2_s::setobj_2_s;
use crate::macros::setsvalue::setsvalue;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;

#[export_name = "luaur_luaD_seterrorobj"]
pub unsafe fn luaD_seterrorobj(l: *mut lua_State, errcode: core::ffi::c_int, oldtop: StkId) {
    if errcode == lua_Status::LUA_ERRMEM as core::ffi::c_int {
        setsvalue!(l, oldtop, luaS_newliteral(l, c"not enough memory".as_ptr()));
    } else if errcode == lua_Status::LUA_ERRERR as core::ffi::c_int {
        setsvalue!(
            l,
            oldtop,
            luaS_newliteral(l, c"error in error handling".as_ptr())
        );
    } else if errcode == lua_Status::LUA_ERRSYNTAX as core::ffi::c_int
        || errcode == lua_Status::LUA_ERRRUN as core::ffi::c_int
    {
        // error message on current top
        setobj_2_s!(l, oldtop, (*l).top.offset(-1));
    }

    (*l).top = oldtop.offset(1);
}
