use crate::enums::lua_type::lua_Type;
use crate::macros::api_incr_top::api_incr_top;
use crate::macros::setnvalue::setnvalue;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_pushnumber"]
pub unsafe fn lua_pushnumber(L: *mut lua_State, n: f64) {
    setnvalue!((*L).top, n);
    api_incr_top!(L);
}
