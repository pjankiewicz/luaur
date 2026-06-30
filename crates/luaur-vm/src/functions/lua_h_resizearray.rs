use crate::functions::adjustasize::adjustasize;
use crate::functions::resize::resize;
use crate::macros::sizenode::sizenode;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::lua_table::LuaTable;

extern "C" {
    #[allow(non_upper_case_globals)]
    #[link_name = "luaur_luaH_dummynode"]
    pub(crate) static luaH_dummynode: crate::records::lua_node::LuaNode;
}

#[allow(non_snake_case)]
pub fn lua_h_resizearray(L: *mut lua_State, t: *mut LuaTable, nasize: i32) {
    unsafe {
        let nsize = if (*t).node == &luaH_dummynode as *const _ as *mut _ {
            0
        } else {
            sizenode!(t)
        };

        let asize = adjustasize(t, nasize, core::ptr::null());

        resize(L, t, asize, nsize);
    }
}

#[export_name = "luaur_luaH_resizearray"]
pub unsafe extern "C" fn lua_h_resizearray_export(
    L: *mut lua_State,
    t: *mut core::ffi::c_void,
    nasize: i32,
) {
    lua_h_resizearray(L, t as *mut LuaTable, nasize);
}
