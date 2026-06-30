use crate::macros::api_incr_top::api_incr_top;
use crate::records::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_luaA_pushvalue"]
pub unsafe fn luaA_pushvalue(L: *mut lua_State, o: *const TValue) {
    *(*L).top = *o;
    api_incr_top!(L);
}
