use crate::functions::lua_l_checkinteger_64::luaL_checkinteger64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_int64_clamp"]
pub unsafe fn int64_clamp(l: *mut lua_State) -> core::ffi::c_int {
    let a = luaL_checkinteger64(l, 1);
    let mi = luaL_checkinteger64(l, 2);
    let mx = luaL_checkinteger64(l, 3);

    luaL_argcheck!(l, mi <= mx, 3, "max must be greater than or equal to min");

    if a < mi {
        lua_pushinteger_64(l, mi);
    } else if a > mx {
        lua_pushinteger_64(l, mx);
    } else {
        lua_pushinteger_64(l, a);
    }

    1
}
