use luaur_vm::macros::lu_tag_iterator::LU_TAG_ITERATOR;
use luaur_vm::macros::setobj::setobj;
use luaur_vm::macros::setpvalue::setpvalue;
use luaur_vm::macros::ttisnil::ttisnil;
use luaur_vm::records::lua_node::LuaNode;
use luaur_vm::records::lua_table::LuaTable;
use luaur_vm::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::t_value::TValue;
use luaur_vm::type_aliases::value::Value;

#[repr(C)]
struct LocalTKey {
    value: Value,
    extra: [core::ffi::c_int; 1],
    tt_next: i32,
}

impl LocalTKey {
    #[inline]
    fn tt(&self) -> core::ffi::c_int {
        self.tt_next & 0xF
    }
}

#[repr(C)]
struct LocalLuaNode {
    val: TValue,
    key: LocalTKey,
}

unsafe fn node_gval(n: *const LuaNode) -> *const TValue {
    core::ptr::addr_of!((*(n as *const LocalLuaNode)).val)
}

unsafe fn get_node_key(L: *mut lua_State, obj: *mut TValue, node: *const LuaNode) {
    let key = core::ptr::addr_of!((*(node as *const LocalLuaNode)).key);
    (*obj).value = (*key).value;
    core::ptr::copy_nonoverlapping(
        (*key).extra.as_ptr(),
        (*obj).extra.as_mut_ptr(),
        (*obj).extra.len(),
    );
    (*obj).tt = (*key).tt();
    luaur_vm::macros::checkliveness::checkliveness!((*L).global, obj);
}

#[allow(non_snake_case)]
pub unsafe fn forg_loop_node_iter(
    L: *mut lua_State,
    h: *mut LuaTable,
    mut index: i32,
    ra: *mut TValue,
) -> bool {
    let sizearray = (*h).sizearray;
    let sizenode = 1 << (*h).lsizenode;

    // then we advance index through the hash portion
    while (index as u32).wrapping_sub(sizearray as u32) < (sizenode as u32) {
        let n = (*h).node.add((index - sizearray) as usize);

        if !ttisnil!(node_gval(n)) {
            setpvalue!(
                ra.add(2),
                (index + 1) as usize as *mut core::ffi::c_void,
                LU_TAG_ITERATOR
            );
            get_node_key(L, ra.add(3), n);
            setobj!(L, ra.add(4), node_gval(n));

            return true;
        }

        index += 1;
    }

    false
}

#[export_name = "luaur_forgLoopNodeIter"]
pub unsafe extern "C" fn forgLoopNodeIter(
    L: *mut lua_State,
    h: *mut core::ffi::c_void,
    index: i32,
    ra: *mut TValue,
) -> bool {
    forg_loop_node_iter(L, h as *mut LuaTable, index, ra)
}
