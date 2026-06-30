use crate::enums::lua_type::lua_Type;
use crate::macros::api_incr_top::api_incr_top;
use crate::macros::setbvalue::setbvalue;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_lua_pushboolean"]
pub unsafe fn lua_pushboolean(L: *mut lua_State, b: core::ffi::c_int) {
    // The setbvalue macro requires TValue and lua_Type to be in scope at the call site.
    setbvalue!((*L).top, b != 0); // ensure that true is 1
    api_incr_top!(L);
}
