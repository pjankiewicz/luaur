use crate::functions::lua_l_checkvector::lua_l_checkvector;
use crate::functions::lua_l_optvector::lua_l_optvector;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_vector_angle"]
pub unsafe fn vector_angle(l: *mut lua_State) -> i32 {
    let a = lua_l_checkvector(l, 1);
    let b = lua_l_optvector(l, 2, core::ptr::null());
    let axis = lua_l_optvector(l, 3, core::ptr::null());

    let a_val = core::slice::from_raw_parts(a, 3);
    let b_val = core::slice::from_raw_parts(b, 3);

    let cross = [
        a_val[1] * b_val[2] - a_val[2] * b_val[1],
        a_val[2] * b_val[0] - a_val[0] * b_val[2],
        a_val[0] * b_val[1] - a_val[1] * b_val[0],
    ];

    let sin_a = ((cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]) as f64).sqrt();
    let cos_a = (a_val[0] * b_val[0] + a_val[1] * b_val[1] + a_val[2] * b_val[2]) as f64;
    let mut angle = sin_a.atan2(cos_a);

    if !axis.is_null() {
        let axis_val = core::slice::from_raw_parts(axis, 3);
        if cross[0] * axis_val[0] + cross[1] * axis_val[1] + cross[2] * axis_val[2] < 0.0 {
            angle = -angle;
        }
    }

    lua_pushnumber(l, angle);
    1
}
