use crate::functions::lua_h_getnum::lua_h_getnum;
use crate::macros::dummynode::dummynode;
use crate::macros::getaboundary::getaboundary;
use crate::macros::ttisnil::ttisnil;
use crate::records::lua_node::LuaNode;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::t_value::TValue;
use core::ffi::c_int;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[inline]
unsafe fn maybesetaboundary(t: *mut LuaTable, boundary: c_int) {
    if (*t).union.aboundary <= 0 {
        (*t).union.aboundary = -boundary;
    }
}

unsafe fn updateaboundary(t: *mut LuaTable, boundary: c_int) -> c_int {
    if boundary < (*t).sizearray && ttisnil!((*t).array.add((boundary - 1) as usize)) {
        if boundary >= 2 && !ttisnil!((*t).array.add((boundary - 2) as usize)) {
            maybesetaboundary(t, boundary - 1);
            return boundary - 1;
        }
    } else if boundary + 1 < (*t).sizearray
        && !ttisnil!((*t).array.add(boundary as usize))
        && ttisnil!((*t).array.add((boundary + 1) as usize))
    {
        maybesetaboundary(t, boundary + 1);
        return boundary + 1;
    }

    0
}

#[allow(non_snake_case)]
pub unsafe fn lua_h_getn(t: *mut LuaTable) -> c_int {
    let boundary = getaboundary(t);

    if boundary > 0 {
        if !ttisnil!((*t).array.add(((*t).sizearray - 1) as usize))
            && (*t).node == dummynode as *mut LuaNode
        {
            return (*t).sizearray;
        }

        if boundary < (*t).sizearray
            && !ttisnil!((*t).array.add((boundary - 1) as usize))
            && ttisnil!((*t).array.add(boundary as usize))
        {
            return boundary;
        }

        let foundboundary = updateaboundary(t, boundary);
        if foundboundary > 0 {
            return foundboundary;
        }
    }

    let j = (*t).sizearray;

    if j > 0 && ttisnil!((*t).array.add((j - 1) as usize)) {
        let mut base: *mut TValue = (*t).array;
        let mut rest = j;

        while rest >> 1 != 0 {
            let half = rest >> 1;
            if !ttisnil!(base.add(half as usize)) {
                base = base.add(half as usize);
            }
            rest -= half;
        }

        let boundary = if !ttisnil!(base) { 1 } else { 0 } + base.offset_from((*t).array) as c_int;
        maybesetaboundary(t, boundary);
        boundary
    } else {
        LUAU_ASSERT!((*t).node == dummynode as *mut LuaNode || ttisnil!(lua_h_getnum(t, j + 1)));
        j
    }
}

#[export_name = "luaur_luaH_getn"]
pub unsafe extern "C" fn lua_h_getn_export(t: *mut core::ffi::c_void) -> c_int {
    lua_h_getn(t as *mut LuaTable)
}

#[allow(unused_imports)]
pub use lua_h_getn as luaH_getn;
