use crate::functions::lua_m_free::luaM_free_;
use crate::functions::lua_m_freegco::luaM_freegco_;
use crate::macros::sizenode::sizenode;
use crate::records::lua_page::lua_Page;
use crate::records::lua_state::lua_State;
use crate::records::lua_table::LuaTable;
use crate::type_aliases::lua_node::LuaNode;
use crate::type_aliases::t_value::TValue;

extern "C" {
    #[allow(non_upper_case_globals)]
    #[link_name = "luaur_luaH_dummynode"]
    pub(crate) static luaH_dummynode: LuaNode;
}

#[allow(non_snake_case)]
pub unsafe fn lua_h_free(L: *mut lua_State, t: *mut LuaTable, page: *mut lua_Page) {
    if (*t).node != &luaH_dummynode as *const _ as *mut LuaNode {
        let size = sizenode!(t);
        luaM_free_(
            L,
            (*t).node as *mut core::ffi::c_void,
            size as usize * core::mem::size_of::<LuaNode>(),
            (*t).memcat,
        );
    }
    if !(*t).array.is_null() {
        luaM_free_(
            L,
            (*t).array as *mut core::ffi::c_void,
            (*t).sizearray as usize * core::mem::size_of::<TValue>(),
            (*t).memcat,
        );
    }
    luaM_freegco_(
        L,
        t as *mut crate::records::gc_object::GCObject,
        core::mem::size_of::<LuaTable>(),
        (*t).memcat,
        page,
    );
}
