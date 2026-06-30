use crate::macros::api_incr_top::api_incr_top;
use crate::macros::setvvalue::setvvalue;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;

#[export_name = "luaur_lua_pushvector_lua_state_f32_f32_f32"]
pub unsafe fn lua_pushvector_lua_state_f32_f32_f32(l: *mut lua_State, x: f32, y: f32, z: f32) {
    setvvalue!((*l).top, x, y, z, 0.0f32);
    api_incr_top!(l);
}
