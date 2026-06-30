use crate::functions::lua_gettop::lua_gettop;
use crate::functions::lua_l_checkinteger_64::lua_l_checkinteger_64;
use crate::functions::lua_pushboolean::lua_pushboolean;
use crate::type_aliases::lua_state::LuaState;

#[export_name = "luaur_int64_btest"]
pub unsafe fn int64_btest(l: *mut LuaState) -> core::ffi::c_int {
    let mut tres: u64 = u64::MAX;
    let n = lua_gettop(l);

    for i in 1..=n {
        let x = lua_l_checkinteger_64(l, i) as u64;
        tres &= x;
    }

    lua_pushboolean(l, if tres != 0 { 1 } else { 0 });

    1
}
