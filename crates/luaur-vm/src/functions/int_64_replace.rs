use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_l_optinteger_64::lua_l_optinteger_64;
use crate::functions::lua_pushinteger_64::lua_pushinteger_64;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::macros::lua_l_error::luaL_error;
use crate::type_aliases::lua_state::LuaState;

// `int64.replace` — overwrite a `w`-bit field at offset `f` of `n` with the
// low `w` bits of `r` (bit-field replacement), with range checks on `f`/`w`.
pub unsafe fn int64_replace(l: *mut LuaState) -> core::ffi::c_int {
    let n = lua_l_checkinteger_64(l, 1);
    let r = lua_l_checkinteger_64(l, 2);
    let f = lua_l_checkinteger_64(l, 3);
    let w = lua_l_optinteger_64(l, 4, 1);

    luaL_argcheck!(l, 0 <= f && f <= 63, 3, "field cannot be negative");
    luaL_argcheck!(l, 0 < w, 4, "width must be positive");
    // `f` is bounded to [0,63] above; compare `w > 64 - f` so a near-i64::MAX
    // width can't overflow the `f + w` addition.
    if w > 64 - f {
        luaL_error!(l, "trying to access non-existent bits");
    }

    let base_mask = 0xFFFFFFFFFFFFFFFFu64 >> (64 - w as u32);
    let replacement = ((r as u64) & base_mask) << (f as u32);
    let mask = 0xFFFFFFFFFFFFFFFFu64 ^ (base_mask << (f as u32));
    lua_pushinteger_64(l, (((n as u64) & mask) | replacement) as i64);

    1
}
