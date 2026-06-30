use crate::functions::lua_l_checkvector::lua_l_checkvector;
use crate::functions::lua_pushvector_lapi::lua_pushvector_lua_state_f32_f32_f32_f32;
use crate::functions::lua_pushvector_lapi_alt_b::lua_pushvector_lua_state_f32_f32_f32;
use crate::functions::luaui_clampf::luaui_clampf;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::macros::lua_vector_size::LUA_VECTOR_SIZE;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_vector_clamp"]
pub unsafe fn vector_clamp(l: *mut lua_State) -> core::ffi::c_int {
    let v = lua_l_checkvector(l, 1);
    let min = lua_l_checkvector(l, 2);
    let max = lua_l_checkvector(l, 3);

    luaL_argcheck!(
        l,
        *min.offset(0) <= *max.offset(0),
        3,
        "max.x must be greater than or equal to min.x"
    );
    luaL_argcheck!(
        l,
        *min.offset(1) <= *max.offset(1),
        3,
        "max.y must be greater than or equal to min.y"
    );
    luaL_argcheck!(
        l,
        *min.offset(2) <= *max.offset(2),
        3,
        "max.z must be greater than or equal to min.z"
    );

    if LUA_VECTOR_SIZE == 4 {
        lua_pushvector_lua_state_f32_f32_f32_f32(
            l,
            luaui_clampf(*v.offset(0), *min.offset(0), *max.offset(0)),
            luaui_clampf(*v.offset(1), *min.offset(1), *max.offset(1)),
            luaui_clampf(*v.offset(2), *min.offset(2), *max.offset(2)),
            luaui_clampf(*v.offset(3), *min.offset(3), *max.offset(3)),
        );
    } else {
        lua_pushvector_lua_state_f32_f32_f32(
            l,
            luaui_clampf(*v.offset(0), *min.offset(0), *max.offset(0)),
            luaui_clampf(*v.offset(1), *min.offset(1), *max.offset(1)),
            luaui_clampf(*v.offset(2), *min.offset(2), *max.offset(2)),
        );
    }

    1
}
