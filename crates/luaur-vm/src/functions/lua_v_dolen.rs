use crate::enums::lua_type::lua_Type;
use crate::functions::call_t_mres::call_t_mres;
use crate::functions::lua_h_getn::lua_h_getn;
use crate::functions::lua_t_gettmbyobj::lua_t_gettmbyobj;
use crate::macros::cast_num::cast_num;
use crate::macros::fasttm::fasttm;
use crate::macros::hvalue::hvalue;
use crate::macros::lua_g_runerror::lua_g_runerror;
use crate::macros::lua_g_typeerror::luaG_typeerror;
use crate::macros::lua_o_nilobject::luaO_nilobject;
use crate::macros::setnvalue::setnvalue;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttisnil::ttisnil;
use crate::macros::ttisnumber::ttisnumber;
use crate::macros::ttype::ttype;
use crate::type_aliases::lua_state::LuaState;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;
use crate::type_aliases::tms::TMS;

#[allow(non_snake_case)]
pub unsafe fn lua_v_dolen(L: *mut LuaState, ra: StkId, rb: *const TValue) {
    let mut tm: *const TValue = core::ptr::null();

    match ttype!(rb) {
        x if x == lua_Type::LUA_TTABLE as i32 => {
            let h = hvalue!(rb);
            tm = fasttm(L, (*h).metatable, TMS::TM_LEN as i32);
            if tm.is_null() {
                // lua_h_getn stub is currently pub fn lua_h_getn();
                // We must cast it to the real signature to call it with the table pointer.
                let lua_h_getn_ptr = lua_h_getn as *const core::ffi::c_void;
                let lua_h_getn_real: unsafe fn(
                    *mut crate::records::lua_table::LuaTable,
                ) -> core::ffi::c_int = core::mem::transmute(lua_h_getn_ptr);
                setnvalue!(ra, cast_num!(lua_h_getn_real(h)));
                return;
            }
        }
        x if x == lua_Type::LUA_TSTRING as i32 => {
            let ts = tsvalue!(rb);
            setnvalue!(ra, cast_num!((*ts).len));
            return;
        }
        _ => {
            tm = lua_t_gettmbyobj(L, rb, TMS::TM_LEN);
        }
    }

    if ttisnil!(tm) {
        luaG_typeerror!(L, rb, c"get length of".as_ptr());
    }

    let res = call_t_mres(L, ra, tm, rb, luaO_nilobject);

    if !ttisnumber!(res) {
        lua_g_runerror!(L, "'__len' must return a number");
    }
}

#[export_name = "luaur_luaV_dolen"]
pub unsafe extern "C" fn lua_v_dolen_export(L: *mut LuaState, ra: StkId, rb: *const TValue) {
    lua_v_dolen(L, ra, rb);
}
