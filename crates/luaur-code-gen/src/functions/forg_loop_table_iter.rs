use luaur_vm::macros::lu_tag_iterator::LU_TAG_ITERATOR;
use luaur_vm::macros::setnvalue::setnvalue;
use luaur_vm::macros::setobj_2_s::setobj2s;
use luaur_vm::macros::ttisnil::ttisnil;
use luaur_vm::records::lua_table::LuaTable;
use luaur_vm::type_aliases::t_value::TValue;

use crate::type_aliases::lua_state::LuaState;

#[allow(non_snake_case)]
pub unsafe fn forg_loop_table_iter(
    L: *mut LuaState,
    h: *mut LuaTable,
    mut index: i32,
    ra: *mut TValue,
) -> bool {
    let sizearray = (*h).sizearray;

    while (index as u32) < (sizearray as u32) {
        let e = (*h).array.add(index as usize);

        if !ttisnil!(e) {
            (*ra.add(2)).value.p = (index + 1) as usize as *mut core::ffi::c_void;
            (*ra.add(2)).tt = LU_TAG_ITERATOR;

            setnvalue!(ra.add(3), (index + 1) as f64);
            setobj2s!(L, ra.add(4), e);

            return true;
        }

        index += 1;
    }

    false
}

#[export_name = "luaur_forgLoopTableIter"]
pub unsafe extern "C" fn forgLoopTableIter(
    L: *mut LuaState,
    h: *mut core::ffi::c_void,
    index: i32,
    ra: *mut TValue,
) -> bool {
    forg_loop_table_iter(L, h as *mut LuaTable, index, ra)
}
