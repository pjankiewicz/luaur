use crate::type_aliases::lua_node::LuaNode;

/// The shared immutable empty-hash sentinel. Reference: `VM/src/ltable.cpp:48`
/// `const LuaNode luaH_dummynode = {{{NULL},{0},LUA_TNIL}, {{NULL},{0},LUA_TNIL,0}};`
///
/// `LuaNode` holds raw pointers so it is not `Sync`; the wrapper asserts what
/// the C++ global guarantees — the object is immutable shared data.
#[repr(transparent)]
pub struct DummyNodeSentinel(pub LuaNode);
unsafe impl Sync for DummyNodeSentinel {}

#[export_name = "luaur_luaH_dummynode"]
#[allow(non_upper_case_globals)]
pub static luaH_dummynode: DummyNodeSentinel = DummyNodeSentinel(LuaNode {
    val: crate::records::lua_t_value::TValue {
        value: crate::type_aliases::value::Value {
            p: core::ptr::null_mut(),
        },
        extra: [0],
        tt: 0, // LUA_TNIL
    },
    key: crate::records::t_key::TKey {
        value: crate::type_aliases::value::Value {
            p: core::ptr::null_mut(),
        },
        extra: [0],
        tt_next: 0, // tt=LUA_TNIL, next=0 (packed)
    },
});

/// C++ `#define dummynode (&luaH_dummynode)`.
#[allow(non_upper_case_globals)]
pub const dummynode: *const LuaNode = &luaH_dummynode.0;

#[allow(non_upper_case_globals)]
pub const DUMMYNODE: *const LuaNode = dummynode;

#[allow(non_upper_case_globals)]
pub const luaH_dummynode_ptr: *const LuaNode = dummynode;
