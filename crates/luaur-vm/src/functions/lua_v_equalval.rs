use crate::enums::lua_type::lua_Type;
use crate::functions::call_t_mres::call_t_mres;
use crate::functions::get_comp_tm::get_comp_tm;
use crate::functions::lua_t_gettmbyobj::lua_t_gettmbyobj;
use crate::functions::luai_veceq::luai_veceq;
use crate::macros::bvalue::bvalue;
use crate::macros::classvalue::classvalue;
use crate::macros::gcvalue::gcvalue;
use crate::macros::hvalue::hvalue;
use crate::macros::l_isfalse::l_isfalse;
use crate::macros::lightuserdatatag::lightuserdatatag;
use crate::macros::luai_inteq::luai_inteq;
use crate::macros::luai_numeq::luai_numeq;
use crate::macros::lvalue::lvalue;
use crate::macros::nvalue::nvalue;
use crate::macros::objectvalue::objectvalue;
use crate::macros::ttisnil::ttisnil;
use crate::macros::ttype::ttype;
use crate::macros::uvalue::uvalue;
use crate::macros::vvalue::vvalue;
use crate::type_aliases::lua_state::LuaState;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_v_equalval(L: *mut LuaState, t1: *const TValue, t2: *const TValue) -> i32 {
    let tm: *const TValue;
    LUAU_ASSERT!(ttype!(t1) == ttype!(t2));

    match ttype!(t1) {
        t if t == lua_Type::LUA_TNIL as i32 => return 1,
        t if t == lua_Type::LUA_TNUMBER as i32 => {
            return if luai_numeq(nvalue!(t1), nvalue!(t2)) {
                1
            } else {
                0
            };
        }
        t if t == lua_Type::LUA_TINTEGER as i32 => {
            return if luai_inteq(lvalue!(t1) as f64, lvalue!(t2) as f64) {
                1
            } else {
                0
            };
        }
        t if t == lua_Type::LUA_TVECTOR as i32 => {
            return if luai_veceq(vvalue!(t1).as_ptr(), vvalue!(t2).as_ptr()) {
                1
            } else {
                0
            };
        }
        t if t == lua_Type::LUA_TBOOLEAN as i32 => {
            return if bvalue!(t1) == bvalue!(t2) { 1 } else { 0 };
        }
        t if t == lua_Type::LUA_TLIGHTUSERDATA as i32 => {
            return if ((*t1).value.p == (*t2).value.p)
                && (lightuserdatatag!(t1) == lightuserdatatag!(t2))
            {
                1
            } else {
                0
            };
        }
        t if t == lua_Type::LUA_TUSERDATA as i32 => {
            let u1 = uvalue!(t1);
            let u2 = uvalue!(t2);
            tm = get_comp_tm(L, (*u1).metatable, (*u2).metatable, TMS::TM_EQ);
            if tm.is_null() {
                return if core::ptr::eq(u1, u2) { 1 } else { 0 };
            }
        }
        t if t == lua_Type::LUA_TCLASS as i32 => {
            return if core::ptr::eq(classvalue!(t1), classvalue!(t2)) {
                1
            } else {
                0
            };
        }
        t if t == lua_Type::LUA_TOBJECT as i32 => {
            let t1inst = objectvalue!(t1);
            let t2inst = objectvalue!(t2);
            if (*t1inst).lclass != (*t2inst).lclass {
                return 0;
            }
            tm = lua_t_gettmbyobj(L, t1, TMS::TM_EQ);
            if ttisnil!(tm) {
                return if core::ptr::eq(t1inst, t2inst) { 1 } else { 0 };
            }
        }
        t if t == lua_Type::LUA_TTABLE as i32 => {
            let h1 = hvalue!(t1);
            let h2 = hvalue!(t2);
            tm = get_comp_tm(L, (*h1).metatable, (*h2).metatable, TMS::TM_EQ);
            if tm.is_null() {
                return if core::ptr::eq(h1, h2) { 1 } else { 0 };
            }
        }
        _ => {
            return if core::ptr::eq(gcvalue!(t1), gcvalue!(t2)) {
                1
            } else {
                0
            };
        }
    }

    call_t_mres(L, (*L).top, tm, t1, t2);
    if !l_isfalse!((*L).top) {
        1
    } else {
        0
    }
}

#[export_name = "luaur_luaV_equalval"]
pub unsafe extern "C" fn lua_v_equalval_export(
    L: *mut LuaState,
    t1: *const TValue,
    t2: *const TValue,
) -> i32 {
    lua_v_equalval(L, t1, t2)
}
