use crate::functions::arrayindex::arrayindex;
use crate::functions::lua_h_getnum::lua_h_getnum;
use crate::macros::dummynode::luaH_dummynode;
use crate::macros::nvalue::nvalue;
use crate::macros::ttisnil::ttisnil;
use crate::macros::ttisnumber::ttisnumber;
use crate::type_aliases::lua_table::LuaTable;
use crate::type_aliases::t_value::TValue;

pub unsafe fn adjustasize(
    t: *mut LuaTable,
    mut size: core::ffi::c_int,
    ek: *const TValue,
) -> core::ffi::c_int {
    let tbound = (*t).node != &luaH_dummynode as *const _ as *mut _ || size < (*t).sizearray;
    let ekindex = if !ek.is_null() && ttisnumber!(ek) {
        arrayindex(nvalue!(ek))
    } else {
        -1
    };

    // move the array size up until the boundary is guaranteed to be inside the array part.
    // Stop at INT_MAX: the array can't be larger, and `size + 1` there overflows `int`
    // (UB in C++ Luau / a panic with overflow-checks on — found by the run fuzz target).
    while size != core::ffi::c_int::MAX
        && (size + 1 == ekindex || (tbound && !ttisnil!(lua_h_getnum(t, size + 1))))
    {
        size += 1;
    }

    size
}
