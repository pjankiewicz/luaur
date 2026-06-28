use crate::enums::lua_type::lua_Type;
use crate::functions::getboolfield::getboolfield;
use crate::functions::getfield::getfield;
use crate::functions::lua_l_checktype::lua_l_checktype;
use crate::functions::lua_pushnil::lua_pushnil;
use crate::functions::lua_pushnumber::lua_pushnumber;
use crate::functions::lua_settop::lua_settop;
use crate::functions::os_timegm::os_timegm;
use crate::functions::os_timegm::time_t;
use crate::functions::os_timegm::tm;
use crate::macros::lua_isnoneornil::lua_isnoneornil;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

unsafe extern "C" {
    fn time(t: *mut time_t) -> time_t;
}

#[no_mangle]
pub unsafe fn os_time(l: *mut lua_State) -> c_int {
    let t: i64;

    if lua_isnoneornil!(l, 1) {
        t = time(core::ptr::null_mut());
    } else {
        let mut ts = tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
        };

        lua_l_checktype(l, 1, lua_Type::LUA_TTABLE as c_int);
        lua_settop(l, 1);

        ts.tm_sec = getfield(l, "sec", 0);
        ts.tm_min = getfield(l, "min", 0);
        ts.tm_hour = getfield(l, "hour", 12);
        ts.tm_mday = getfield(l, "day", -1);
        // wrapping_sub avoids `int` underflow on an INT_MIN month/year (UB in
        // C++; panic with overflow-checks). os_timegm widens to i64 and the
        // `t == -1` path rejects out-of-range dates, so a wrapped field can't UB.
        ts.tm_mon = getfield(l, "month", -1).wrapping_sub(1);
        ts.tm_year = getfield(l, "year", -1).wrapping_sub(1900);
        ts.tm_isdst = getboolfield(l, "isdst");

        t = os_timegm(&ts);
    }

    if t == -1 {
        lua_pushnil(l);
    } else {
        lua_pushnumber(l, t as f64);
    }

    1
}
