use crate::enums::lua_status::lua_Status;
use crate::functions::lua_checkstack::lua_checkstack;
use crate::functions::lua_l_error_l::lua_l_error_l;
use crate::functions::lua_rawcheckstack::lua_rawcheckstack;
use crate::functions::lua_xmove::lua_xmove;
use crate::macros::cast_int::cast_int;
use crate::macros::co_status_error::CO_STATUS_ERROR;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_auxresumecont"]
pub unsafe fn auxresumecont(L: *mut lua_State, co: *mut lua_State) -> core::ffi::c_int {
    if (*co).status == lua_Status::LUA_OK as u8 || (*co).status == lua_Status::LUA_YIELD as u8 {
        let nres = cast_int!((*co).top.offset_from((*co).base));
        if lua_checkstack(L, nres + 1) == 0 {
            lua_l_error_l(
                L,
                c"too many results to resume".as_ptr(),
                format_args!("too many results to resume"),
            );
        }
        lua_xmove(co, L, nres);
        nres
    } else {
        lua_rawcheckstack(L, 2);
        lua_xmove(co, L, 1);
        CO_STATUS_ERROR
    }
}
