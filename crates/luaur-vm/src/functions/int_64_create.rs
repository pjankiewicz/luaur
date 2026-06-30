use crate::functions::lua_l_checknumber::lua_l_checknumber;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_create"]
pub unsafe fn int64_create(l: *mut LuaState) -> core::ffi::c_int {
    let x = lua_l_checknumber(l, 1);

    // C++: if (x >= -9223372036854775808.0 && x < 9223372036854775808.0)
    // These constants are exactly -2^63 and 2^63.
    if x >= -9223372036854775808.0 && x < 9223372036854775808.0 {
        let val = x as i64;

        // C++: if (((double)l) == x)
        if (val as f64) == x {
            lua_pushinteger_64(l, val);
            return 1;
        }
    }

    lua_pushnil(l);
    1
}
