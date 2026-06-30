use crate::enums::lua_type::lua_Type;
use crate::functions::lua_g_readonlyerror::lua_g_readonlyerror;
use crate::functions::lua_h_getn::lua_h_getn;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_settop::lua_settop;
use crate::functions::lua_v_lessthan::lua_v_lessthan;
use crate::functions::sort_func::sort_func;
use crate::functions::sort_rec::sort_rec;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_isnoneornil::lua_isnoneornil;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::lua_table::LuaTable;
use crate::type_aliases::sort_predicate::SortPredicate;
use core::ffi::c_int;

#[export_name = "luaur_tsort"]
pub unsafe fn tsort(L: *mut lua_State) -> c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as c_int);

    let t = hvalue!((*L).base) as *mut LuaTable;
    let n = lua_h_getn(t);

    if (*t).readonly != 0 {
        lua_g_readonlyerror(L);
    }

    let mut pred: SortPredicate = Some(lua_v_lessthan);
    if !lua_isnoneornil!(L, 2) {
        lua_l_checktype(L, 2, lua_Type::LUA_TFUNCTION as c_int);
        pred = Some(sort_func);
    }
    lua_settop(L, 2);

    if n > 0 {
        sort_rec(L, t, 0, n - 1, n, pred);
    }
    0
}
