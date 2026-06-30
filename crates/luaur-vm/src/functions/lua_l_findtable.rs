use crate::enums::lua_type::lua_Type;
use crate::functions::lua_createtable::lua_createtable;
use crate::functions::lua_pushlstring::lua_pushlstring;
use crate::functions::lua_pushvalue::lua_pushvalue;
use crate::functions::lua_rawget::lua_rawget;
use crate::functions::lua_remove::lua_remove;
use crate::functions::lua_settable::lua_settable;
use crate::functions::lua_type::lua_type;
use crate::macros::lua_pop::lua_pop;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::{c_char, c_int};

/// `const char* luaL_findtable(lua_State* L, int idx, const char* fname, int szhint)`
///
/// C++ source: `VM/src/laux.cpp:330`
#[export_name = "luaur_luaL_findtable"]
pub unsafe fn luaL_findtable(
    l: *mut lua_State,
    idx: c_int,
    mut fname: *const c_char,
    szhint: c_int,
) -> *const c_char {
    extern "C" {
        fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
        fn strlen(s: *const c_char) -> usize;
    }

    lua_pushvalue(l, idx);
    loop {
        let mut e = strchr(fname, '.' as c_int);
        if e.is_null() {
            e = fname.add(strlen(fname)) as *mut c_char;
        }

        let len = (e as *const c_char).offset_from(fname) as usize;
        lua_pushlstring(l, fname, len);
        lua_rawget(l, -2);

        if lua_type(l, -1) == (lua_Type::LUA_TNIL as i32) {
            lua_pop(l, 1); // remove this nil
            let next_szhint = if *e == ('.' as c_char) { 1 } else { szhint };
            lua_createtable(l, 0, next_szhint);
            lua_pushlstring(l, fname, len);
            lua_pushvalue(l, -2);
            lua_settable(l, -4);
        } else if lua_type(l, -1) != (lua_Type::LUA_TTABLE as i32) {
            lua_pop(l, 2); // remove table and value
            return fname;
        }

        lua_remove(l, -2); // remove previous table
        fname = e.add(1) as *const c_char;

        if *e != ('.' as c_char) {
            break;
        }
    }

    core::ptr::null()
}
