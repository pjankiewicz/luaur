use crate::enums::lua_type::lua_Type;
use crate::functions::lua_a_pushvalue::luaA_pushvalue;
use crate::functions::lua_h_clone::lua_h_clone;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_getmetafield::lua_l_getmetafield;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_l_argcheck::luaL_argcheck;
use crate::macros::sethvalue::sethvalue;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::t_value::TValue;
use core::ffi::c_int;

#[export_name = "luaur_tclone"]
pub unsafe fn tclone(L: *mut lua_State) -> c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as c_int);

    luaL_argcheck!(
        L,
        lua_l_getmetafield(L, 1, c"__metatable".as_ptr()) == 0,
        1,
        "table has a protected metatable"
    );

    let tt = lua_h_clone(L, hvalue!((*L).base));

    let mut v: TValue = core::mem::zeroed();
    sethvalue!(L, &mut v, tt);
    luaA_pushvalue(L, &v);

    1
}
