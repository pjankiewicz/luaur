use crate::enums::lua_type::lua_Type;
use crate::macros::api_incr_top::api_incr_top;
use crate::macros::setnilvalue::setnilvalue;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_pushnil"]
pub unsafe fn lua_pushnil(l: *mut lua_State) {
    setnilvalue!((*l).top);
    api_incr_top!(l);
}
