//! Node: `cxx:Function:Luau.VM:VM/src/lvmutils.cpp:182:luaV_settable`
//! Source: `VM/src/lvmutils.cpp:182-240` (hand-ported)

use crate::functions::call_tm::call_tm;
use crate::functions::lua_g_indexerror::luaG_indexerror;
use crate::functions::lua_g_missingmembererror::luaG_missingmembererror;
use crate::functions::lua_g_readonlyerror::luaG_readonlyerror;
use crate::functions::lua_h_get::lua_h_get;
use crate::functions::lua_t_gettmbyobj::lua_t_gettmbyobj;
use crate::macros::fasttm::fasttm;
use crate::macros::gval_2_slot::gval2slot;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_c_barrier::luaC_barrier;
use crate::macros::lua_c_barriert::luaC_barriert;
use crate::macros::lua_g_runerror::lua_g_runerror;
use crate::macros::lua_h_setslot::luaH_setslot;
use crate::macros::lua_r_lookupmemberatoffset::luaR_lookupmemberatoffset;
use crate::macros::maxtagloop::MAXTAGLOOP;
use crate::macros::nvalue::nvalue;
use crate::macros::objectvalue::objectvalue;
use crate::macros::setobj::setobj;
use crate::macros::setobj_2_class::setobj2class;
use crate::macros::setobj_2_t::setobj2t;
use crate::macros::ttisfunction::ttisfunction;
use crate::macros::ttisnil::ttisnil;
use crate::macros::ttisobject::ttisobject;
use crate::macros::ttistable::ttistable;
use crate::records::luau_object::LuauObject;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_v_settable(
    L: *mut lua_State,
    mut t: *const TValue,
    key: *mut TValue,
    val: StkId,
) {
    let mut temp: TValue = core::mem::zeroed();
    let mut loop_ = 0;
    while loop_ < MAXTAGLOOP {
        let mut tm: *const TValue = core::ptr::null();
        if ttistable!(t) {
            let h = hvalue!(t);

            let oldval = lua_h_get(h, key as *const TValue);

            if ttisnil!(oldval) {
                tm = fasttm(L, (*h).metatable, TMS::TM_NEWINDEX as i32);
            }

            if !ttisnil!(oldval) || tm.is_null() {
                if (*h).readonly != 0 {
                    luaG_readonlyerror(L);
                }

                let newval = luaH_setslot!(L, h, oldval, key as *const TValue);

                (*L).cachedslot = gval2slot!(h, newval as *const TValue);

                setobj2t!(L, newval, val as *const TValue);
                luaC_barriert!(L, h, val as *const TValue);
                return;
            }
        } else if luaur_common::FFlag::DebugLuauUserDefinedClassesRuntime.get() && ttisobject!(t) {
            let inst = &mut **objectvalue!(t) as *mut LuauObject;
            let offset = lua_h_get((*(*inst).lclass).memberstooffset, key as *const TValue);
            if ttisnil!(offset) {
                luaG_missingmembererror(L, t, key as *const TValue);
            }
            let offsetnum = nvalue!(offset) as i32;
            LUAU_ASSERT!(offsetnum >= 0 && offsetnum < (*(*inst).lclass).numberofallmembers);
            if offsetnum >= (*(*inst).lclass).numberofinstancemembers {
                luaG_indexerror(L, t, key as *const TValue);
            }
            setobj2class!(
                L,
                (*inst).members.add(offsetnum as usize),
                val as *const TValue
            );
            luaC_barrier!(L, inst, val as *const TValue);
            return;
        } else {
            tm = lua_t_gettmbyobj(L, t, TMS::TM_NEWINDEX);
            if ttisnil!(tm) {
                luaG_indexerror(L, t, key as *const TValue);
            }
        }

        if ttisfunction!(tm) {
            call_tm(L, tm, t, key as *const TValue, val as *const TValue);
            return;
        }
        setobj!(L, &mut temp as *mut TValue, tm);
        t = &temp as *const TValue;
        loop_ += 1;
    }
    lua_g_runerror!(L, "'__newindex' chain too long; possible loop");
}

#[export_name = "luaur_luaV_settable"]
pub unsafe extern "C" fn lua_v_settable_export(
    L: *mut lua_State,
    t: *const TValue,
    key: *mut TValue,
    val: StkId,
) {
    lua_v_settable(L, t, key, val);
}

#[allow(non_snake_case, unused_imports)]
pub use lua_v_settable as luaV_settable;
