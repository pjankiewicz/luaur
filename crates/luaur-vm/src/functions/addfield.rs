//! Node: `cxx:Function:Luau.VM:VM/src/ltablib.cpp:230:addfield`
//!
//! Helper for `table.concat` — append element `i` of table `t` to the buffer.
//! Fast path reads a string directly from the array part; otherwise it goes
//! through `lua_rawgeti` and rejects non-string/number values.

use crate::enums::lua_type::lua_Type;
use crate::functions::lua_l_addlstring::lua_l_addlstring;
use crate::functions::lua_l_addvalue::lua_l_addvalue;
use crate::functions::lua_l_typename::lua_l_typename;
use crate::functions::lua_rawgeti::lua_rawgeti;
use crate::macros::getstr::getstr;
use crate::macros::lua_l_error::luaL_error;
use crate::macros::tsvalue::tsvalue;
use crate::macros::ttisstring::ttisstring;
use crate::records::lua_l_strbuf::LuaLStrbuf;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

pub fn addfield(L: *mut lua_State, b: *mut LuaLStrbuf, i: i32, t: *mut LuaTable) {
    unsafe {
        // C++ does `cast_to(unsigned, i - 1)` here; for i = INT_MIN the `i - 1`
        // is signed-overflow UB upstream (ltablib.cpp:232). wrapping_sub matches
        // the two's-complement value C++ relies on: it wraps to INT_MAX, fails the
        // `< sizearray` bound, and falls through to the rawgeti slow path.
        if !t.is_null()
            && (i.wrapping_sub(1) as u32) < (*t).sizearray as u32
            && ttisstring!((*t).array.add(i.wrapping_sub(1) as usize))
        {
            let ts = tsvalue!((*t).array.add((i - 1) as usize));
            lua_l_addlstring(b, getstr(ts), (*ts).len as usize);
        } else {
            let tt = lua_rawgeti(L, 1, i);
            if tt != lua_Type::LUA_TSTRING as c_int && tt != lua_Type::LUA_TNUMBER as c_int {
                let tn = core::ffi::CStr::from_ptr(lua_l_typename(L, -1)).to_string_lossy();
                luaL_error!(
                    L,
                    "invalid value ({}) at index {} in table for 'concat'",
                    tn,
                    i
                );
            }
            lua_l_addvalue(b);
        }
    }
}
