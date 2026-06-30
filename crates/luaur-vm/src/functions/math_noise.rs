use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::functions::lua_tonumberx::lua_tonumberx;
use crate::macros::lua_isnoneornil::lua_isnoneornil;
use crate::macros::lua_l_argexpected::luaL_argexpected;
use crate::type_aliases::lua_state::lua_State;
use luaur_common::FFlag::FixMathNoisePrecision;

#[export_name = "luaur_math_noise"]
pub unsafe fn math_noise(L: *mut lua_State) -> i32 {
    let mut nx = 0;
    let mut ny = 0;
    let mut nz = 0;

    let x = lua_tonumberx(L, 1, &mut nx);
    let y = lua_tonumberx(L, 2, &mut ny);
    let z = lua_tonumberx(L, 3, &mut nz);

    luaL_argexpected!(L, nx != 0, 1, "number");
    luaL_argexpected!(L, ny != 0 || lua_isnoneornil!(L, 2), 2, "number");
    luaL_argexpected!(L, nz != 0 || lua_isnoneornil!(L, 3), 3, "number");

    let x = if FixMathNoisePrecision.get() {
        let x_mod = x % 256.0;
        if x_mod < 0.0 {
            x_mod + 256.0
        } else {
            x_mod
        }
    } else {
        x
    };

    let y = if FixMathNoisePrecision.get() {
        let y_mod = y % 256.0;
        if y_mod < 0.0 {
            y_mod + 256.0
        } else {
            y_mod
        }
    } else {
        y
    };

    let z = if FixMathNoisePrecision.get() {
        let z_mod = z % 256.0;
        if z_mod < 0.0 {
            z_mod + 256.0
        } else {
            z_mod
        }
    } else {
        z
    };

    let r = crate::functions::perlin::perlin(x as f32, y as f32, z as f32);

    lua_pushnumber(L, r as f64);
    1
}
