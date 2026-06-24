//! Thin, centralized imports of the luaur C API we build on.
//!
//! Every raw `lua_*` function/type/constant used by `luaur-rt` is re-exported
//! from here, so the rest of the crate has a single place to look and we keep
//! the (long) import paths in one module. Nothing in here is part of the public
//! API.

pub(crate) use core::ffi::{c_char, c_int, c_void};

// ---- types ---------------------------------------------------------------
pub(crate) use luaur_vm::type_aliases::lua_c_function::lua_CFunction;
pub(crate) use luaur_vm::type_aliases::lua_state::lua_State;

// ---- state ---------------------------------------------------------------
pub(crate) use luaur_vm::functions::lua_close::lua_close;
pub(crate) use luaur_vm::functions::lua_l_newstate::lua_l_newstate;
pub(crate) use luaur_vm::functions::lua_l_openlibs::lua_l_openlibs;

// ---- stack / values ------------------------------------------------------
pub(crate) use luaur_vm::functions::lua_gettop::lua_gettop;
pub(crate) use luaur_vm::functions::lua_pushboolean::lua_pushboolean;
pub(crate) use luaur_vm::functions::lua_pushlstring::lua_pushlstring;
pub(crate) use luaur_vm::functions::lua_pushnil::lua_pushnil;
pub(crate) use luaur_vm::functions::lua_pushnumber::lua_pushnumber;
pub(crate) use luaur_vm::functions::lua_pushvalue::lua_pushvalue;
pub(crate) use luaur_vm::functions::lua_settop::lua_settop;
pub(crate) use luaur_vm::functions::lua_toboolean::lua_toboolean;
pub(crate) use luaur_vm::functions::lua_tolstring::lua_tolstring;
pub(crate) use luaur_vm::functions::lua_tonumberx::lua_tonumberx;
pub(crate) use luaur_vm::functions::lua_type::lua_type;

// ---- tables --------------------------------------------------------------
pub(crate) use luaur_vm::functions::lua_createtable::lua_createtable;
pub(crate) use luaur_vm::functions::lua_gettable::lua_gettable;
pub(crate) use luaur_vm::functions::lua_next::lua_next;
pub(crate) use luaur_vm::functions::lua_objlen::lua_objlen;
pub(crate) use luaur_vm::functions::lua_settable::lua_settable;

// ---- raw table access + metatables + stack juggling ----------------------
pub(crate) use luaur_vm::functions::lua_equal::lua_equal;
pub(crate) use luaur_vm::functions::lua_getmetatable::lua_getmetatable;
pub(crate) use luaur_vm::functions::lua_getreadonly::lua_getreadonly;
pub(crate) use luaur_vm::functions::lua_insert::lua_insert;
pub(crate) use luaur_vm::functions::lua_rawget::lua_rawget;
pub(crate) use luaur_vm::functions::lua_rawset::lua_rawset;
pub(crate) use luaur_vm::functions::lua_setreadonly::lua_setreadonly;
pub(crate) use luaur_vm::functions::lua_topointer::lua_topointer;

// ---- garbage collection --------------------------------------------------
pub(crate) use luaur_vm::enums::lua_gc_op::lua_GCOp;
pub(crate) use luaur_vm::functions::lua_gc::lua_gc;

// ---- metatable-aware tostring --------------------------------------------
pub(crate) use luaur_vm::functions::lua_l_tolstring::lua_l_tolstring;

// ---- closures / userdata -------------------------------------------------
pub(crate) use luaur_vm::functions::lua_newuserdatadtor::lua_newuserdatadtor;
pub(crate) use luaur_vm::functions::lua_pushcclosurek::lua_pushcclosurek;
pub(crate) use luaur_vm::functions::lua_setmetatable::lua_setmetatable;
pub(crate) use luaur_vm::functions::lua_touserdata::lua_touserdata;

// ---- refs / call / load --------------------------------------------------
pub(crate) use luaur_vm::functions::lua_checkstack::lua_checkstack;
pub(crate) use luaur_vm::functions::lua_error::lua_error;
pub(crate) use luaur_vm::functions::lua_pcall::lua_pcall;
pub(crate) use luaur_vm::functions::lua_ref::lua_ref;
pub(crate) use luaur_vm::functions::lua_unref::lua_unref;
pub(crate) use luaur_vm::functions::luau_load::luau_load;

// ---- macro-defined helpers (exposed as plain fns) ------------------------
pub(crate) use luaur_vm::macros::lua_globalsindex::LUA_GLOBALSINDEX;
pub(crate) use luaur_vm::macros::lua_pop::lua_pop;
pub(crate) use luaur_vm::macros::lua_upvalueindex::lua_upvalueindex;

/// Lua type tags (subset we care about). The VM returns these as `c_int` from
/// [`lua_type`]; we keep our own constants to avoid leaking the VM enum.
pub(crate) mod ttype {
    use super::c_int;
    pub const NONE: c_int = -1;
    pub const NIL: c_int = 0;
    pub const BOOLEAN: c_int = 1;
    pub const NUMBER: c_int = 3;
    pub const STRING: c_int = 6;
    pub const TABLE: c_int = 7;
    pub const FUNCTION: c_int = 8;
    pub const USERDATA: c_int = 9;
}
