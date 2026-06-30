use crate::enums::lua_type::lua_Type;
use crate::functions::call_bin_tm::call_bin_tm;
use crate::functions::lua_g_aritherror::luaG_aritherror;
use crate::functions::lua_v_tonumber::lua_v_tonumber;
use crate::functions::luai_numidiv::luai_numidiv;
use crate::functions::luai_nummod::luai_nummod;
use crate::macros::cast_to::cast_to;
use crate::macros::luai_numadd::luai_numadd;
use crate::macros::luai_numdiv::luai_numdiv;
use crate::macros::luai_nummul::luai_nummul;
use crate::macros::luai_numpow::luai_numpow;
use crate::macros::luai_numsub::luai_numsub;
use crate::macros::luai_numunm::luai_numunm;
use crate::macros::nvalue::nvalue;
use crate::macros::setnvalue::setnvalue;
use crate::macros::setvvalue::setvvalue;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttisvector::ttisvector;
use crate::macros::vvalue::vvalue;
use crate::records::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[allow(non_snake_case)]
pub unsafe fn lua_v_doarithimpl(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
    op: TMS,
) {
    let mut tempb = TValue::default();
    let mut tempc = TValue::default();
    let b: *const TValue;
    let c: *const TValue;

    let vb = if ttisvector!(rb) {
        vvalue!(rb).as_ptr()
    } else {
        core::ptr::null()
    };
    let vc = if ttisvector!(rc) {
        vvalue!(rc).as_ptr()
    } else {
        core::ptr::null()
    };

    if !vb.is_null() && !vc.is_null() {
        match op {
            TMS::TM_ADD => {
                setvvalue!(
                    ra,
                    *vb.add(0) + *vc.add(0),
                    *vb.add(1) + *vc.add(1),
                    *vb.add(2) + *vc.add(2),
                    *vb.add(3) + *vc.add(3)
                );
                return;
            }
            TMS::TM_SUB => {
                setvvalue!(
                    ra,
                    *vb.add(0) - *vc.add(0),
                    *vb.add(1) - *vc.add(1),
                    *vb.add(2) - *vc.add(2),
                    *vb.add(3) - *vc.add(3)
                );
                return;
            }
            TMS::TM_MUL => {
                setvvalue!(
                    ra,
                    *vb.add(0) * *vc.add(0),
                    *vb.add(1) * *vc.add(1),
                    *vb.add(2) * *vc.add(2),
                    *vb.add(3) * *vc.add(3)
                );
                return;
            }
            TMS::TM_DIV => {
                setvvalue!(
                    ra,
                    *vb.add(0) / *vc.add(0),
                    *vb.add(1) / *vc.add(1),
                    *vb.add(2) / *vc.add(2),
                    *vb.add(3) / *vc.add(3)
                );
                return;
            }
            TMS::TM_IDIV => {
                setvvalue!(
                    ra,
                    luai_numidiv(*vb.add(0) as f64, *vc.add(0) as f64) as f32,
                    luai_numidiv(*vb.add(1) as f64, *vc.add(1) as f64) as f32,
                    luai_numidiv(*vb.add(2) as f64, *vc.add(2) as f64) as f32,
                    luai_numidiv(*vb.add(3) as f64, *vc.add(3) as f64) as f32
                );
                return;
            }
            TMS::TM_UNM => {
                setvvalue!(ra, -*vb.add(0), -*vb.add(1), -*vb.add(2), -*vb.add(3));
                return;
            }
            _ => {}
        }
    } else if !vb.is_null() {
        let c_ptr = if ttisnumber!(rc) {
            rc
        } else {
            lua_v_tonumber(rc, &mut tempc)
        };
        if !c_ptr.is_null() {
            let nc = cast_to!(f32, nvalue!(c_ptr));
            match op {
                TMS::TM_MUL => {
                    setvvalue!(
                        ra,
                        *vb.add(0) * nc,
                        *vb.add(1) * nc,
                        *vb.add(2) * nc,
                        *vb.add(3) * nc
                    );
                    return;
                }
                TMS::TM_DIV => {
                    setvvalue!(
                        ra,
                        *vb.add(0) / nc,
                        *vb.add(1) / nc,
                        *vb.add(2) / nc,
                        *vb.add(3) / nc
                    );
                    return;
                }
                TMS::TM_IDIV => {
                    setvvalue!(
                        ra,
                        luai_numidiv(*vb.add(0) as f64, nc as f64) as f32,
                        luai_numidiv(*vb.add(1) as f64, nc as f64) as f32,
                        luai_numidiv(*vb.add(2) as f64, nc as f64) as f32,
                        luai_numidiv(*vb.add(3) as f64, nc as f64) as f32
                    );
                    return;
                }
                _ => {}
            }
        }
    } else if !vc.is_null() {
        let b_ptr = if ttisnumber!(rb) {
            rb
        } else {
            lua_v_tonumber(rb, &mut tempb)
        };
        if !b_ptr.is_null() {
            let nb = cast_to!(f32, nvalue!(b_ptr));
            match op {
                TMS::TM_MUL => {
                    setvvalue!(
                        ra,
                        nb * *vc.add(0),
                        nb * *vc.add(1),
                        nb * *vc.add(2),
                        nb * *vc.add(3)
                    );
                    return;
                }
                TMS::TM_DIV => {
                    setvvalue!(
                        ra,
                        nb / *vc.add(0),
                        nb / *vc.add(1),
                        nb / *vc.add(2),
                        nb / *vc.add(3)
                    );
                    return;
                }
                TMS::TM_IDIV => {
                    setvvalue!(
                        ra,
                        luai_numidiv(nb as f64, *vc.add(0) as f64) as f32,
                        luai_numidiv(nb as f64, *vc.add(1) as f64) as f32,
                        luai_numidiv(nb as f64, *vc.add(2) as f64) as f32,
                        luai_numidiv(nb as f64, *vc.add(3) as f64) as f32
                    );
                    return;
                }
                _ => {}
            }
        }
    }

    let b_res = lua_v_tonumber(rb, &mut tempb);
    let c_res = lua_v_tonumber(rc, &mut tempc);
    if !b_res.is_null() && !c_res.is_null() {
        let nb = nvalue!(b_res);
        let nc = nvalue!(c_res);
        match op {
            TMS::TM_ADD => setnvalue!(ra, luai_numadd(nb, nc)),
            TMS::TM_SUB => setnvalue!(ra, luai_numsub(nb, nc)),
            TMS::TM_MUL => setnvalue!(ra, luai_nummul(nb, nc)),
            TMS::TM_DIV => setnvalue!(ra, luai_numdiv(nb, nc)),
            TMS::TM_IDIV => setnvalue!(ra, luai_numidiv(nb, nc)),
            TMS::TM_MOD => setnvalue!(ra, luai_nummod(nb, nc)),
            TMS::TM_POW => setnvalue!(ra, luai_numpow(nb, nc)),
            TMS::TM_UNM => setnvalue!(ra, luai_numunm(nb)),
            _ => LUAU_ASSERT!(false),
        }
    } else if call_bin_tm(L, rb, rc, ra, op) == 0 {
        luaG_aritherror(L, rb, rc, op);
    }
}

#[export_name = "luaur_luaV_doarithimpl_TM_ADD"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_add(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_ADD);
}

#[export_name = "luaur_luaV_doarithimpl_TM_SUB"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_sub(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_SUB);
}

#[export_name = "luaur_luaV_doarithimpl_TM_MUL"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_mul(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_MUL);
}

#[export_name = "luaur_luaV_doarithimpl_TM_DIV"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_div(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_DIV);
}

#[export_name = "luaur_luaV_doarithimpl_TM_IDIV"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_idiv(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_IDIV);
}

#[export_name = "luaur_luaV_doarithimpl_TM_MOD"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_mod(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_MOD);
}

#[export_name = "luaur_luaV_doarithimpl_TM_POW"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_pow(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_POW);
}

#[export_name = "luaur_luaV_doarithimpl_TM_UNM"]
pub unsafe extern "C" fn lua_v_doarithimpl_tm_unm(
    L: *mut lua_State,
    ra: StkId,
    rb: *const TValue,
    rc: *const TValue,
) {
    lua_v_doarithimpl(L, ra, rb, rc, TMS::TM_UNM);
}
