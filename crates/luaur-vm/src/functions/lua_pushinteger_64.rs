use crate::enums::lua_type::lua_Type;
use crate::macros::api_incr_top::api_incr_top;
use crate::macros::setlvalue::setlvalue;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_pushinteger_64"]
pub unsafe fn lua_pushinteger_64(L: *mut lua_State, n: i64) {
    let _ = lua_Type::LUA_TINTEGER;
    setlvalue!((*L).top, n);
    api_incr_top!(L);
}
