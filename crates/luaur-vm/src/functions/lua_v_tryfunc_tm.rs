use crate::functions::lua_t_gettmbyobj::lua_t_gettmbyobj;
use crate::macros::lua_g_typeerror::luaG_typeerror;
use crate::macros::setobj_2_s::setobj2s;
use crate::macros::ttisfunction::ttisfunction;
use crate::records::lua_state::lua_State;
use crate::type_aliases::lua_state::LuaState;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;
use luaur_common::macros::luau_noinline::LUAU_NOINLINE;

#[allow(non_snake_case)]
#[export_name = "luaur_lua_v_tryfunc_tm"]
pub unsafe fn lua_v_tryfunc_tm(L: *mut LuaState, func: StkId) {
    let tm = lua_t_gettmbyobj(
        L as *mut lua_State,
        func,
        crate::type_aliases::tms::TMS::TM_CALL,
    );
    if !ttisfunction!(tm) {
        luaG_typeerror!(L as *mut lua_State, func, c"call".as_ptr());
    }

    let mut p = (*L).top;
    while p > func {
        setobj2s!(L as *mut lua_State, p, p.wrapping_sub(1));
        p = p.wrapping_sub(1);
    }

    (*L).top = (*L).top.wrapping_add(1);
    setobj2s!(L as *mut lua_State, func, tm);
}
