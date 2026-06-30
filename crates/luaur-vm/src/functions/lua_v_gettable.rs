//! Node: `cxx:Function:Luau.VM:VM/src/lvmutils.cpp:102:luaV_gettable`
//! Source: `VM/src/lvmutils.cpp:102-180` (hand-ported)

use crate::functions::call_t_mres::call_t_mres;
use crate::functions::lua_g_indexerror::luaG_indexerror;
use crate::functions::lua_g_missingmembererror::luaG_missingmembererror;
use crate::functions::lua_h_get::lua_h_get;
use crate::functions::lua_t_gettmbyobj::lua_t_gettmbyobj;
use crate::macros::classvalue::classvalue;
use crate::macros::fasttm::fasttm;
use crate::macros::gval_2_slot::gval2slot;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_g_runerror::lua_g_runerror;
use crate::macros::lua_o_nilobject::luaO_nilobject;
use crate::macros::lua_r_lookupmemberatoffset::luaR_lookupmemberatoffset;
use crate::macros::maxtagloop::MAXTAGLOOP;
use crate::macros::nvalue::nvalue;
use crate::macros::objectvalue::objectvalue;
use crate::macros::setobj_2_s::setobj2s;
use crate::macros::ttisclass::ttisclass;
use crate::macros::ttisfunction::ttisfunction;
use crate::macros::ttisnil::ttisnil;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttisobject::ttisobject;
use crate::macros::ttistable::ttistable;
use crate::records::luau_class::LuauClass;
use crate::records::luau_object::LuauObject;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_v_gettable(
    L: *mut lua_State,
    mut t: *const TValue,
    key: *mut TValue,
    val: StkId,
) {
    let mut loop_ = 0;
    while loop_ < MAXTAGLOOP {
        let mut tm: *const TValue;
        if ttistable!(t) {
            let h = hvalue!(t);

            let res = lua_h_get(h, key as *const TValue);

            if res != luaO_nilobject {
                (*L).cachedslot = gval2slot!(h, res);
            }

            if !ttisnil!(res) {
                setobj2s!(L, val, res);
                return;
            }

            tm = fasttm(L, (*h).metatable, TMS::TM_INDEX as i32);
            if tm.is_null() {
                setobj2s!(L, val, res);
                return;
            }
        } else if luaur_common::FFlag::DebugLuauUserDefinedClassesRuntime.get() && ttisobject!(t) {
            let inst = &mut **objectvalue!(t) as *mut LuauObject;
            let offsettval = lua_h_get((*(*inst).lclass).memberstooffset, key as *const TValue);

            if ttisnil!(offsettval) {
                luaG_missingmembererror(L, t, key as *const TValue);
            }

            LUAU_ASSERT!(ttisnumber!(offsettval));
            let offset = nvalue!(offsettval) as i32;
            setobj2s!(L, val, luaR_lookupmemberatoffset!(inst, offset));
            return;
        } else if luaur_common::FFlag::DebugLuauUserDefinedClassesRuntime.get() && ttisclass!(t) {
            let lco = &mut **classvalue!(t) as *mut LuauClass;
            let res = lua_h_get((*lco).memberstooffset, key as *const TValue);

            if ttisnil!(res) {
                luaG_missingmembererror(L, t, key as *const TValue);
            }

            LUAU_ASSERT!(ttisnumber!(res));
            let offset = nvalue!(res) as i32;
            LUAU_ASSERT!(offset >= 0 && offset < (*lco).numberofallmembers);

            if offset < (*lco).numberofinstancemembers {
                luaG_missingmembererror(L, t, key as *const TValue);
            }

            setobj2s!(
                L,
                val,
                (*lco)
                    .staticmembers
                    .add((offset - (*lco).numberofinstancemembers) as usize)
            );
            return;
        } else {
            tm = lua_t_gettmbyobj(L, t, TMS::TM_INDEX);
            if ttisnil!(tm) {
                luaG_indexerror(L, t, key as *const TValue);
            }
        }

        if ttisfunction!(tm) {
            call_t_mres(L, val, tm, t, key as *const TValue);
            return;
        }
        t = tm;
        loop_ += 1;
    }
    lua_g_runerror!(L, "'__index' chain too long; possible loop");
}

#[export_name = "luaur_luaV_gettable"]
pub unsafe extern "C" fn lua_v_gettable_export(
    L: *mut lua_State,
    t: *const TValue,
    key: *mut TValue,
    val: StkId,
) {
    lua_v_gettable(L, t, key, val);
}

#[allow(non_snake_case, unused_imports)]
pub use lua_v_gettable as luaV_gettable;
