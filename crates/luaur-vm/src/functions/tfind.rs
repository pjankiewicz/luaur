use crate::enums::lua_type::lua_Type;
use crate::functions::lua_h_getnum::lua_h_getnum;
use crate::functions::lua_l_checkany::lua_l_checkany;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_l_optinteger::lua_l_optinteger;
use crate::functions::lua_pushinteger::lua_pushinteger;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::macros::equalobj::equalobj;
use crate::macros::hvalue::hvalue;
use crate::macros::ttisnil::ttisnil;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::stk_id::StkId;
use crate::type_aliases::t_value::TValue;

#[no_mangle]
#[allow(non_snake_case)]
pub unsafe fn tfind(L: *mut lua_State) -> core::ffi::c_int {
    lua_l_checktype(L, 1, lua_Type::LUA_TTABLE as core::ffi::c_int);
    lua_l_checkany(L, 2);
    let init = lua_l_optinteger(L, 3, 1);
    if init < 1 {
        // The dependency card for lua_l_argerror_l shows it takes &str.
        crate::functions::lua_l_argerror_l::lua_l_argerror_l(L, 3, "index out of range");
    }

    let t = hvalue!((*L).base);

    let mut i = init;
    loop {
        let e: *const TValue = lua_h_getnum(t, i);
        if ttisnil!(e) {
            break;
        }

        let v: StkId = (*L).base.offset(1);

        if equalobj!(L, v, e) {
            lua_pushinteger(L, i);
            return 1;
        }
        // C++ does `i++` unconditionally; if the table has an element at INT_MAX
        // that doesn't match, the increment is signed-overflow UB (upstream
        // ltablib.cpp:533). There is no valid index past INT_MAX, so stop cleanly.
        if i == core::ffi::c_int::MAX {
            break;
        }
        i += 1;
    }

    lua_pushnil(L);
    1
}
