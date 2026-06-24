//! The [`Lua`] handle and the shared inner state.
//!
//! ## Lifetime model (mirrors mlua's `Rc<inner> + registry-key` design)
//!
//! [`Lua`] owns the `*mut lua_State`. The state is wrapped in an [`Rc`]
//! ([`LuaInner`]) so that long-lived handles ([`Table`], [`Function`],
//! [`LuaString`], the corresponding [`Value`] variants, userdata) can hold a
//! clone of that `Rc` and keep the state alive for as long as they exist.
//!
//! Each such handle additionally holds a **registry reference** obtained via
//! [`lua_ref`] (luaur's `lua_ref`/`lua_unref`). That keeps the underlying Lua
//! value reachable by the GC, and lets the handle re-push the value onto the
//! stack on demand. On `Drop` the handle releases its registry slot with
//! [`lua_unref`] — but only if the state is still alive (the `Rc` keeps it so).
//!
//! `Lua` is single-threaded (`Rc`, so `!Send`/`!Sync`), matching mlua's
//! non-`Send` default.

use std::cell::Cell;
use std::rc::Rc;

use crate::error::{Error, Result};
use crate::ffi::*;
use crate::value::Value;

/// The reference-counted, shared interior of a [`Lua`] instance.
///
/// Held by [`Lua`] and cloned into every long-lived handle. When the last
/// `Rc<LuaInner>` is dropped, [`Drop`] closes the `lua_State`.
pub(crate) struct LuaInner {
    /// The owned VM state pointer. Never null while this `LuaInner` exists.
    pub(crate) state: *mut lua_State,
    /// Whether this `LuaInner` is responsible for closing the state. The
    /// trampoline builds a *borrowed* [`Lua`] around the calling thread's
    /// state and must not close it.
    owned: bool,
}

impl Drop for LuaInner {
    fn drop(&mut self) {
        if self.owned && !self.state.is_null() {
            unsafe { lua_close(self.state) }
        }
    }
}

/// A handle to a Lua interpreter.
///
/// Mirrors `mlua::Lua`. Cloning produces another handle to the **same** VM
/// (the inner state is shared via `Rc`), exactly like mlua.
#[derive(Clone)]
pub struct Lua {
    pub(crate) inner: Rc<LuaInner>,
}

impl Lua {
    /// Create a new Lua state with the standard library opened.
    ///
    /// Mirrors `mlua::Lua::new`.
    pub fn new() -> Lua {
        // luaur's v11+ bytecode needs the default Luau flags on (see the
        // umbrella crate's `eval`).
        luaur_common::set_all_flags(true);
        unsafe {
            let state = lua_l_newstate();
            assert!(!state.is_null(), "lua_l_newstate returned null");
            lua_l_openlibs(state);
            Lua {
                inner: Rc::new(LuaInner { state, owned: true }),
            }
        }
    }

    /// Create a new Lua state **without** opening the standard library.
    ///
    /// A deliberate deviation from mlua (which exposes `StdLib` flags); a
    /// minimal convenience for embedders who want a clean global table.
    pub fn new_empty() -> Lua {
        luaur_common::set_all_flags(true);
        let state = lua_l_newstate();
        assert!(!state.is_null(), "lua_l_newstate returned null");
        Lua {
            inner: Rc::new(LuaInner { state, owned: true }),
        }
    }

    /// The raw state pointer. Internal use only.
    #[inline]
    pub(crate) fn state(&self) -> *mut lua_State {
        self.inner.state
    }

    /// Wrap an *already-existing* state (e.g. the thread passed into a C
    /// trampoline) in a borrowed [`Lua`] that will **not** close it on drop.
    ///
    /// # Safety
    /// `state` must be a valid `lua_State` that outlives the returned handle
    /// and all handles cloned from it.
    pub(crate) unsafe fn from_borrowed(state: *mut lua_State) -> Lua {
        Lua {
            inner: Rc::new(LuaInner {
                state,
                owned: false,
            }),
        }
    }

    /// Register a value sitting at stack index `idx` in the registry and return
    /// a [`LuaRef`] that owns the slot. Does not pop the value.
    pub(crate) fn register_ref(&self, idx: c_int) -> LuaRef {
        let id = unsafe { lua_ref(self.state(), idx) };
        LuaRef {
            inner: self.inner.clone(),
            id: Cell::new(id),
        }
    }

    /// Pop the top stack value and register it, returning a [`LuaRef`].
    pub(crate) fn pop_ref(&self) -> LuaRef {
        let r = self.register_ref(-1);
        unsafe { lua_pop(self.state(), 1) };
        r
    }
}

impl Default for Lua {
    fn default() -> Self {
        Lua::new()
    }
}

// ---------------------------------------------------------------------------
// Public, mlua-style construction API.
// ---------------------------------------------------------------------------

use crate::callback::{create_callback_function, BoxedCallback};
use crate::chunk::Chunk;
use crate::function::Function;
use crate::multi::MultiValue;
use crate::string::LuaString;
use crate::table::Table;
use crate::traits::{FromLuaMulti, IntoLuaMulti};
use crate::userdata::{AnyUserData, UserData};

impl Lua {
    /// The globals table.
    ///
    /// Mirrors `mlua::Lua::globals`. Returns a [`Table`] handle to the global
    /// environment (the table reachable at `LUA_GLOBALSINDEX`).
    pub fn globals(&self) -> Table {
        let state = self.state();
        unsafe {
            // Push the globals table (a copy of the LUA_GLOBALSINDEX pseudo
            // value) and take a ref to it.
            lua_pushvalue(state, LUA_GLOBALSINDEX);
            Table::from_ref(self.pop_ref())
        }
    }

    /// Create a new, empty table.
    ///
    /// Mirrors `mlua::Lua::create_table` (infallible here, so no `Result`
    /// wrapper is strictly needed — but we also provide the `_result` variant
    /// for signature parity below).
    pub fn create_table(&self) -> Table {
        crate::table::create_table(self)
    }

    /// `Result`-returning alias of [`Lua::create_table`] for mlua signature
    /// parity.
    pub fn create_table_result(&self) -> Result<Table> {
        Ok(self.create_table())
    }

    /// Create a Lua string from bytes/str.
    ///
    /// Mirrors `mlua::Lua::create_string`.
    pub fn create_string(&self, s: impl AsRef<[u8]>) -> LuaString {
        crate::string::create_string(self, s.as_ref())
    }

    /// Create a table and populate it from an iterator of key/value pairs.
    ///
    /// Mirrors `mlua::Lua::create_table_from`.
    pub fn create_table_from<K, V, I>(&self, iter: I) -> Result<Table>
    where
        K: crate::traits::IntoLua,
        V: crate::traits::IntoLua,
        I: IntoIterator<Item = (K, V)>,
    {
        let t = self.create_table();
        for (k, v) in iter {
            t.raw_set(k, v)?;
        }
        Ok(t)
    }

    /// Create a sequence (1-based array) table from an iterator of values.
    ///
    /// Mirrors `mlua::Lua::create_sequence_from`.
    pub fn create_sequence_from<V, I>(&self, iter: I) -> Result<Table>
    where
        V: crate::traits::IntoLua,
        I: IntoIterator<Item = V>,
    {
        let t = self.create_table();
        for (i, v) in iter.into_iter().enumerate() {
            t.raw_set((i + 1) as i64, v)?;
        }
        Ok(t)
    }

    /// Run a full garbage-collection cycle.
    ///
    /// Mirrors `mlua::Lua::gc_collect` (infallible here — luaur's `lua_gc`
    /// cannot fail for `collect`).
    pub fn gc_collect(&self) -> Result<()> {
        lua_gc(self.state(), lua_GCOp::LUA_GCCOLLECT as c_int, 0);
        Ok(())
    }

    /// Create a Lua function from a Rust closure.
    ///
    /// Mirrors `mlua::Lua::create_function`. The closure receives `&Lua` and
    /// the arguments converted via [`FromLuaMulti`]; its `Ok` return is
    /// converted via [`IntoLuaMulti`]. Returning `Err` (or panicking) surfaces
    /// as a catchable Lua error.
    pub fn create_function<F, A, R>(&self, func: F) -> Result<Function>
    where
        F: Fn(&Lua, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let boxed: BoxedCallback = Box::new(move |lua, args| {
            let a = A::from_lua_multi(args, lua)?;
            let r = func(lua, a)?;
            r.into_lua_multi(lua)
        });
        create_callback_function(self, boxed)
    }

    /// Create userdata wrapping a `T: UserData` value.
    ///
    /// Mirrors `mlua::Lua::create_userdata`.
    pub fn create_userdata<T: UserData + 'static>(&self, data: T) -> Result<AnyUserData> {
        crate::userdata::create_userdata(self, data)
    }

    /// Load a chunk of Lua source for execution.
    ///
    /// Mirrors `mlua::Lua::load`. Returns a [`Chunk`]; finalize with
    /// [`Chunk::exec`] / [`Chunk::eval`] / [`Chunk::into_function`].
    pub fn load(&self, source: impl AsRef<str>) -> Chunk {
        Chunk {
            lua: self.clone(),
            source: source.as_ref().to_string(),
            name: "chunk".to_string(),
        }
    }

    /// Convert a Rust value into a single Lua [`Value`].
    ///
    /// Mirrors `mlua::Lua::pack`-ish convenience. Provided so callers can build
    /// `Value`s without importing the trait.
    pub fn pack(&self, value: impl crate::traits::IntoLua) -> Result<crate::value::Value> {
        value.into_lua(self)
    }

    /// Build a [`MultiValue`] from anything `IntoLuaMulti`.
    pub fn pack_multi(&self, values: impl IntoLuaMulti) -> Result<MultiValue> {
        values.into_lua_multi(self)
    }
}

/// An owned registry reference to a Lua value.
///
/// Keeps both the value reachable (registry slot) and the VM alive (the cloned
/// `Rc<LuaInner>`). On drop it releases the slot via [`lua_unref`].
pub(crate) struct LuaRef {
    inner: Rc<LuaInner>,
    id: Cell<c_int>,
}

impl LuaRef {
    /// The owning [`Lua`] handle (a fresh borrow sharing the same inner state).
    pub(crate) fn lua(&self) -> Lua {
        Lua {
            inner: self.inner.clone(),
        }
    }

    /// The raw state pointer this ref belongs to.
    #[inline]
    pub(crate) fn state(&self) -> *mut lua_State {
        self.inner.state
    }

    /// The registry id. (Retained for internal diagnostics; handle identity is
    /// established via `lua_topointer`, not the registry slot id.)
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn id(&self) -> c_int {
        self.id.get()
    }

    /// Push the referenced value back onto the stack.
    pub(crate) fn push(&self) {
        // The registry table lives at LUA_REGISTRYINDEX; `lua_ref` stores
        // values keyed by their integer id, so a `rawgeti` on the registry
        // recovers them. luaur exposes this through getfield on the registry
        // via the same mechanism `lua_getref` uses in upstream Luau:
        // `lua_rawgeti(L, LUA_REGISTRYINDEX, id)`.
        unsafe {
            luaur_vm::functions::lua_rawgeti::lua_rawgeti(
                self.state(),
                luaur_vm::macros::lua_registryindex::LUA_REGISTRYINDEX,
                self.id.get(),
            );
        }
    }
}

impl Clone for LuaRef {
    fn clone(&self) -> Self {
        // Re-push the value and take a fresh registry slot, so each clone owns
        // an independent slot (simplest correct behavior).
        self.push();
        let new = self.lua().pop_ref();
        new
    }
}

impl Drop for LuaRef {
    fn drop(&mut self) {
        let id = self.id.get();
        // Only unref live, real slots.
        if id > 0 && !self.inner.state.is_null() {
            unsafe { lua_unref(self.inner.state, id) };
        }
    }
}

impl Lua {
    /// Convenience: convert a top-of-stack value (at `idx`) into a [`Value`],
    /// taking a registry ref for reference types. Does not pop.
    pub(crate) fn value_from_stack(&self, idx: c_int) -> Result<Value> {
        crate::value::value_from_stack(self, idx)
    }

    /// Push a [`Value`] onto the stack.
    pub(crate) fn push_value(&self, value: &Value) -> Result<()> {
        crate::value::push_value(self, value)
    }

    /// Metatable-aware `tostring` of a [`Value`] (honors `__tostring`),
    /// mirroring Lua's `tostring`/`luaL_tolstring`.
    pub(crate) fn value_to_string(&self, value: &Value) -> Result<String> {
        let state = self.state();
        unsafe {
            self.push_value(value)?;
            let mut len = 0usize;
            let p = lua_l_tolstring(state, -1, &mut len);
            let out = if p.is_null() {
                String::new()
            } else {
                let bytes = core::slice::from_raw_parts(p as *const u8, len);
                String::from_utf8_lossy(bytes).into_owned()
            };
            // luaL_tolstring pushes the result string; pop it plus the value.
            lua_pop(state, 2);
            Ok(out)
        }
    }

    /// Map a `lua_pcall`/`luau_load` status code plus the error object on the
    /// stack into an [`Error`]. Assumes a non-zero status and that the error
    /// object is on top of the stack; pops it.
    pub(crate) fn pop_error(&self, _status: c_int) -> Error {
        let state = self.state();
        unsafe {
            let mut len = 0usize;
            let s = lua_tolstring(state, -1, &mut len);
            let msg = if s.is_null() {
                "<non-string error>".to_string()
            } else {
                let bytes = core::slice::from_raw_parts(s as *const u8, len);
                String::from_utf8_lossy(bytes).into_owned()
            };
            lua_pop(state, 1);
            Error::RuntimeError(msg)
        }
    }
}
