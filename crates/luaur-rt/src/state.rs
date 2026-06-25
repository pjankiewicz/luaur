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
//!
//! ## The `send` feature
//!
//! Under the `send` feature (mirroring mlua) the shared interior uses
//! [`XRc`] = `Arc` instead of `Rc`, and [`LuaInner`] / [`LuaRef`] carry a
//! documented `unsafe impl Send`. That makes [`Lua`] and every handle `Send` so
//! the whole VM can be **moved** to another thread. It is *not* made `Sync`: the
//! VM is still single-threaded, the user must serialize all access, and only the
//! ownership *transfer* crosses threads (exactly mlua's `send` contract).

use std::cell::Cell;

use crate::error::{Error, Result};
use crate::sys::*;
use crate::sync::{MaybeSend, MaybeSync, NotSync, XRc, XWeak, NOT_SYNC};
use crate::value::Value;

// Re-export the GC-control types here so they live at `luaur_rt::state::{..}`,
// matching mlua's `mlua::state::{GcMode, GcIncParams, GcGenParams}` path.
pub use crate::gc::{GcGenParams, GcIncParams, GcMode};

/// The reference-counted, shared interior of a [`Lua`] instance.
///
/// Held by [`Lua`] and cloned into every long-lived handle. When the last
/// `XRc<LuaInner>` is dropped, [`Drop`] closes the `lua_State`.
pub(crate) struct LuaInner {
    /// The owned VM state pointer. Never null while this `LuaInner` exists.
    pub(crate) state: *mut lua_State,
    /// Whether this `LuaInner` is responsible for closing the state. The
    /// trampoline builds a *borrowed* [`Lua`] around the calling thread's
    /// state and must not close it.
    owned: bool,
    /// Host type definitions accumulated via [`Lua::add_definitions`] (the
    /// `typecheck` feature), in Luau definition-file syntax. Each registration
    /// is appended separated by a newline; the whole buffer is fed to the
    /// type-checker by [`Lua::check`] / [`Chunk::check`]. Uses the crate's
    /// `RefCell` interior-mutability idiom (the VM is single-threaded).
    #[cfg(feature = "typecheck")]
    typecheck_defs: std::cell::RefCell<String>,
}

impl LuaInner {
    /// Build a fresh `LuaInner`, initializing every field (including the
    /// feature-gated `typecheck_defs` store). Used by all `Lua` constructors so
    /// the field set stays in one place.
    fn new(state: *mut lua_State, owned: bool) -> LuaInner {
        LuaInner {
            state,
            owned,
            #[cfg(feature = "typecheck")]
            typecheck_defs: std::cell::RefCell::new(String::new()),
        }
    }
}

impl Drop for LuaInner {
    fn drop(&mut self) {
        if self.owned && !self.state.is_null() {
            // Drop this VM's application-data store before closing the state
            // (it is keyed by the global-state pointer, still valid here).
            crate::app_data::clear_app_data(self.state);
            unsafe {
                // Reset the active memory category to 0 ("main") before closing.
                // `Lua::set_memory_category` may have left a non-main category
                // active; allocations made during teardown would otherwise be
                // accounted to it, tripping `close_state`'s debug invariant that
                // only category 0 is non-empty at shutdown.
                crate::sys::lua_setmemcat(self.state, 0);
                lua_close(self.state)
            }
        }
    }
}

// Under the `send` feature, allow a `Lua` (and every handle, transitively) to be
// **moved** across threads. The raw `*mut lua_State` is `!Send`/`!Sync` by
// default; these impls encode luaur-rt's documented contract — single-threaded
// *use*, only *ownership transfer* across threads, never concurrent access.
//
// `Send` is the property we actually expose. `Sync` is needed only as an
// internal obligation: `XRc<LuaInner>` is `Arc<LuaInner>` under the feature, and
// `Arc<T>: Send` requires `T: Send + Sync`. We therefore mark `LuaInner` (the
// non-public interior) `Sync`, and then keep the *public* `Lua`/handle types
// `!Sync` with a `NotSync` phantom marker (see [`NotSync`]). Net effect: the VM
// can be moved across threads but never shared/accessed concurrently — exactly
// mlua's `send` contract, minus mlua's extra `Sync` (luaur-rt stays `!Sync`).
#[cfg(feature = "send")]
unsafe impl Send for LuaInner {}
#[cfg(feature = "send")]
unsafe impl Sync for LuaInner {}

/// A handle to a Lua interpreter.
///
/// Mirrors `mlua::Lua`. Cloning produces another handle to the **same** VM
/// (the inner state is shared via `Rc`), exactly like mlua.
#[derive(Clone)]
pub struct Lua {
    pub(crate) inner: XRc<LuaInner>,
    /// Keeps `Lua` `!Sync` under the `send` feature (the VM is move-only, never
    /// shareable). A zero-sized `()` under the default build. See [`NotSync`].
    pub(crate) _not_sync: NotSync,
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
                inner: XRc::new(LuaInner::new(state, true)),
                _not_sync: NOT_SYNC,
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
            inner: XRc::new(LuaInner::new(state, true)),
            _not_sync: NOT_SYNC,
        }
    }

    /// Create a new Lua state with the standard library opened, **without** the
    /// extra safety restrictions a safe `Lua::new` would impose.
    ///
    /// Mirrors `mlua::Lua::unsafe_new`. In Luau there is no separate set of
    /// "unsafe" base libraries (the `debug`/`ffi`/`package` distinction is a
    /// Lua-5.x concept), so this is equivalent to [`Lua::new`]; it exists for
    /// mlua signature parity.
    ///
    /// # Safety
    /// Provided for parity with mlua's `unsafe_new`, which can open libraries
    /// that allow loading native code. luaur's Luau base library does not expose
    /// such facilities, so this is in practice as safe as [`Lua::new`]; the
    /// `unsafe` marker is retained to match mlua's signature.
    pub unsafe fn unsafe_new() -> Lua {
        Lua::new()
    }

    /// Create a new Lua state opening the libraries selected by `libs`, with the
    /// behavioral `options`. Mirrors `mlua::Lua::new_with`.
    ///
    /// **DEVIATION:** luaur opens the Luau base libraries as a unit, so any
    /// non-empty `libs` opens the full standard library and [`StdLib::NONE`]
    /// opens nothing (see [`StdLib`]). `options` is recorded on the VM (currently
    /// only `catch_rust_panics` is observable).
    pub fn new_with(libs: crate::options::StdLib, options: crate::options::LuaOptions) -> Result<Lua> {
        luaur_common::set_all_flags(true);
        unsafe {
            let state = lua_l_newstate();
            assert!(!state.is_null(), "lua_l_newstate returned null");
            if !libs.is_none() {
                lua_l_openlibs(state);
            }
            let lua = Lua {
                inner: XRc::new(LuaInner::new(state, true)),
                _not_sync: NOT_SYNC,
            };
            lua.set_catch_rust_panics(options.catch_rust_panics);
            Ok(lua)
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
            inner: XRc::new(LuaInner::new(state, false)),
            _not_sync: NOT_SYNC,
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

impl Lua {
    /// A non-owning, weak handle to this VM. Mirrors `mlua::Lua::weak`.
    ///
    /// The [`WeakLua`] does not keep the VM alive; it can be upgraded back to a
    /// strong [`Lua`] only while at least one strong handle still exists.
    pub fn weak(&self) -> WeakLua {
        WeakLua(XRc::downgrade(&self.inner))
    }
}

/// A weak handle to a [`Lua`] instance. Mirrors `mlua::WeakLua`.
///
/// Holds a non-owning reference to the shared VM interior; upgrade it to a
/// strong [`Lua`] with [`WeakLua::try_upgrade`] / [`WeakLua::upgrade`].
#[derive(Clone)]
pub struct WeakLua(pub(crate) XWeak<LuaInner>);

impl WeakLua {
    /// Try to obtain a strong [`Lua`] handle. Returns `None` if the VM has
    /// already been destroyed. Mirrors `mlua::WeakLua::try_upgrade`.
    pub fn try_upgrade(&self) -> Option<Lua> {
        self.0.upgrade().map(|inner| Lua {
            inner,
            _not_sync: NOT_SYNC,
        })
    }

    /// Obtain a strong [`Lua`] handle, panicking if the VM has been destroyed.
    /// Mirrors `mlua::WeakLua::upgrade`.
    pub fn upgrade(&self) -> Lua {
        self.try_upgrade().expect("Lua instance is destroyed")
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
        F: Fn(&Lua, A) -> Result<R> + MaybeSend + 'static,
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

    /// Create a Lua function from a Rust **mutable** closure.
    ///
    /// Mirrors `mlua::Lua::create_function_mut`. The closure is guarded by a
    /// [`RefCell`](std::cell::RefCell); a re-entrant call (the callback running
    /// Lua that calls the same callback again) surfaces as
    /// [`Error::RecursiveMutCallback`](crate::Error::RecursiveMutCallback)
    /// rather than allowing mutable aliasing.
    pub fn create_function_mut<F, A, R>(&self, func: F) -> Result<Function>
    where
        F: FnMut(&Lua, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let func = std::cell::RefCell::new(func);
        self.create_function(move |lua, args| {
            let mut borrow = func
                .try_borrow_mut()
                .map_err(|_| Error::RecursiveMutCallback)?;
            (borrow)(lua, args)
        })
    }

    /// Create userdata wrapping a `T: UserData` value.
    ///
    /// Mirrors `mlua::Lua::create_userdata`.
    pub fn create_userdata<T: UserData + MaybeSend + MaybeSync + 'static>(
        &self,
        data: T,
    ) -> Result<AnyUserData> {
        crate::userdata::create_userdata(self, data)
    }

    /// Create a Lua function from a Rust **async** closure (the `async`
    /// feature).
    ///
    /// Mirrors `mlua::Lua::create_async_function`. The closure receives an owned
    /// [`Lua`] and the converted arguments, and returns a `Future`. When the
    /// resulting Lua function is called, it runs on a coroutine that **yields**
    /// while the future is pending; a driver such as
    /// [`Function::call_async`](crate::Function::call_async) /
    /// [`Chunk::eval_async`](crate::Chunk::eval_async) resumes the coroutine,
    /// polls the future, and resumes it with the result when ready.
    ///
    /// The executor is provided by the caller (luaur-rt is executor-agnostic,
    /// exactly like mlua): the returned futures must be `.await`ed / polled on
    /// the caller's runtime (e.g. tokio).
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn create_async_function<F, A, FR, R>(&self, func: F) -> Result<Function>
    where
        F: Fn(Lua, A) -> FR + 'static,
        A: FromLuaMulti,
        FR: std::future::Future<Output = Result<R>> + 'static,
        R: IntoLuaMulti,
    {
        let callback: crate::async_support::AsyncCallback = Box::new(move |lua, args| {
            // Convert the arguments eagerly; defer the conversion error into the
            // future so it surfaces uniformly on the first poll.
            let a = A::from_lua_multi(args, &lua);
            let fut = a.map(|a| func(lua.clone(), a));
            Box::pin(async move {
                let r = fut?.await?;
                r.into_lua_multi(&lua)
            })
        });
        crate::async_support::create_async_callback(self, callback)
    }

    /// Creates and returns a Luau [buffer] object from a byte slice of data.
    ///
    /// Mirrors `mlua::Lua::create_buffer`.
    ///
    /// [buffer]: https://luau.org/library#buffer-library
    pub fn create_buffer(&self, data: impl AsRef<[u8]>) -> Result<crate::buffer::Buffer> {
        let data = data.as_ref();
        let buffer = self.create_buffer_with_capacity(data.len())?;
        if !data.is_empty() {
            buffer.write_bytes(0, data);
        }
        Ok(buffer)
    }

    /// Creates and returns a Luau [buffer] object with the specified size.
    ///
    /// Size limit is 1GB. All bytes are initialized to zero. Exceeding the
    /// limit returns a `RuntimeError` carrying a `"memory allocation error"`
    /// message (matching mlua).
    ///
    /// Mirrors `mlua::Lua::create_buffer_with_capacity`.
    ///
    /// [buffer]: https://luau.org/library#buffer-library
    pub fn create_buffer_with_capacity(&self, size: usize) -> Result<crate::buffer::Buffer> {
        crate::buffer::create_buffer_with_capacity(self, size)
    }

    /// Creates and returns a Luau [`Vector`](crate::Vector) value.
    ///
    /// Mirrors `mlua::Lua::create_vector`. luaur is a 3-wide vector build.
    pub fn create_vector(&self, x: f32, y: f32, z: f32) -> crate::vector::Vector {
        crate::vector::Vector::new(x, y, z)
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
            environment: None,
            compiler: None,
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

    /// Convert any `FromLuaMulti` from a packed [`MultiValue`]. Mirrors
    /// `mlua::Lua::unpack_multi` (and `unpack` for the single-value case).
    pub fn unpack_multi<T: FromLuaMulti>(&self, values: MultiValue) -> Result<T> {
        T::from_lua_multi(values, self)
    }

    /// Convert a single Lua [`Value`] to a Rust value. Mirrors `mlua::Lua::unpack`.
    pub fn unpack<T: crate::traits::FromLua>(&self, value: Value) -> Result<T> {
        T::from_lua(value, self)
    }

    /// Coerce a [`Value`] to an integer the way Lua's `tonumber`+integer check
    /// would (`"1"` -> `Some(1)`, `"1.5"` -> `None`, a non-numeric value ->
    /// `None`). Mirrors `mlua::Lua::coerce_integer`.
    pub fn coerce_integer(&self, value: Value) -> Result<Option<crate::value::Integer>> {
        let state = self.state();
        unsafe {
            self.push_value(&value)?;
            let mut isnum: c_int = 0;
            let n = lua_tonumberx(state, -1, &mut isnum);
            lua_pop(state, 1);
            if isnum == 0 {
                return Ok(None);
            }
            // An integral, in-range float coerces to an integer; otherwise None.
            if n.fract() == 0.0 && n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Ok(Some(n as i64))
            } else {
                Ok(None)
            }
        }
    }

    /// Coerce a [`Value`] to a float the way Lua's `tonumber` would. Mirrors
    /// `mlua::Lua::coerce_number`.
    pub fn coerce_number(&self, value: Value) -> Result<Option<crate::value::Number>> {
        let state = self.state();
        unsafe {
            self.push_value(&value)?;
            let mut isnum: c_int = 0;
            let n = lua_tonumberx(state, -1, &mut isnum);
            lua_pop(state, 1);
            if isnum == 0 {
                Ok(None)
            } else {
                Ok(Some(n))
            }
        }
    }

    /// Replace the global environment with `globals`. Mirrors
    /// `mlua::Lua::set_globals`.
    ///
    /// In a sandboxed Lua state the globals table is read-only and cannot be
    /// replaced; this returns a [`Error::RuntimeError`] in that case (matching
    /// mlua / Luau).
    pub fn set_globals(&self, globals: Table) -> Result<()> {
        if self.is_sandboxed() {
            return Err(Error::runtime(
                "cannot change globals in a sandboxed Lua state",
            ));
        }
        let state = self.state();
        unsafe {
            globals.push_to_stack();
            lua_replace(state, LUA_GLOBALSINDEX);
        }
        Ok(())
    }

    /// Build a stack traceback string for this VM. Mirrors `mlua::Lua::traceback`.
    ///
    /// `msg`, if present, is prepended to the traceback; `level` selects the
    /// starting stack level. The returned [`LuaString`] holds the traceback as
    /// produced by `luaL_traceback`.
    pub fn traceback(&self, msg: Option<&str>, level: usize) -> Result<LuaString> {
        let state = self.state();
        unsafe {
            lua_l_traceback(state, state, msg, level as c_int);
            // luaL_traceback pushes the resulting string onto the stack.
            Ok(LuaString::from_ref(self.pop_ref()))
        }
    }
}

// ---------------------------------------------------------------------------
// Static type-checking (the `typecheck` feature).
//
// luaur ships Luau's static type checker, so — unlike mlua — a script can be
// type-checked against the host surface *before* it runs. The host surface is
// described in Luau definition-file syntax and accumulated on the `Lua` via
// `add_definitions`; `check` / `Chunk::check` then validate source against it.
// ---------------------------------------------------------------------------
#[cfg(feature = "typecheck")]
#[cfg_attr(docsrs, doc(cfg(feature = "typecheck")))]
impl Lua {
    /// Register host type `definitions` (Luau definition-file syntax) so later
    /// [`Lua::check`] / [`Chunk::check`] calls type-check against them.
    ///
    /// `definitions` describes the host-provided globals — the Rust functions,
    /// values, and userdata you expose to the runtime (e.g. via
    /// [`Lua::create_function`] / [`UserData`](crate::UserData)):
    ///
    /// ```text
    /// declare function add(a: number, b: number): number
    /// declare config: { name: string, retries: number }
    /// ```
    ///
    /// The definitions are validated before being recorded: if they are
    /// malformed, this returns [`Error::TypeError`](crate::Error::TypeError)
    /// carrying the (`in_definitions`) diagnostics and records nothing. On
    /// success they are appended to this VM's accumulated definitions.
    pub fn add_definitions(&self, defs: &str) -> Result<()> {
        // Validate the new definitions in isolation by checking a trivial body.
        if let Err(diagnostics) = crate::typecheck::check_with_definitions("return nil", defs) {
            // Only the definition-side diagnostics are this call's fault; a
            // type error in the trivial body would be ours, not the caller's.
            let def_errors: Vec<crate::TypeDiagnostic> = diagnostics
                .into_iter()
                .filter(|d| d.in_definitions)
                .collect();
            if !def_errors.is_empty() {
                return Err(Error::TypeError(def_errors));
            }
        }
        // Append, newline-separated, to the accumulated definitions.
        let mut store = self.inner.typecheck_defs.borrow_mut();
        if !store.is_empty() {
            store.push('\n');
        }
        store.push_str(defs);
        Ok(())
    }

    /// Type-check `source` against this VM's accumulated host definitions.
    ///
    /// Returns `Ok(())` if the source type-checks clean, or
    /// [`Error::TypeError`](crate::Error::TypeError) carrying the structured
    /// diagnostics otherwise.
    ///
    /// The Luau VM is dynamically typed, so this is **advisory**: a script that
    /// fails the check can still be run (`exec`/`eval`). The value is catching
    /// host-API misuse statically, before running untrusted or generated code.
    pub fn check(&self, source: &str) -> Result<()> {
        let defs = self.inner.typecheck_defs.borrow();
        let result = if defs.is_empty() {
            crate::typecheck::check(source)
        } else {
            crate::typecheck::check_with_definitions(source, &defs)
        };
        result.map_err(Error::TypeError)
    }

    /// Type-check `source` against this VM's accumulated host definitions **plus**
    /// the extra `defs` (for a one-off check that does not persist `defs`).
    ///
    /// Same mapping as [`Lua::check`]: `Ok(())` when clean, otherwise
    /// [`Error::TypeError`](crate::Error::TypeError).
    pub fn check_with_definitions(&self, source: &str, defs: &str) -> Result<()> {
        let accumulated = self.inner.typecheck_defs.borrow();
        let combined = if accumulated.is_empty() {
            defs.to_string()
        } else {
            format!("{accumulated}\n{defs}")
        };
        crate::typecheck::check_with_definitions(source, &combined).map_err(Error::TypeError)
    }
}

/// An owned registry reference to a Lua value.
///
/// Keeps both the value reachable (registry slot) and the VM alive (the cloned
/// `XRc<LuaInner>`). On drop it releases the slot via [`lua_unref`].
pub(crate) struct LuaRef {
    inner: XRc<LuaInner>,
    id: Cell<c_int>,
}

// `LuaRef` is shared behind `XRc<LuaRef>` (`Arc<LuaRef>` under the feature) by
// every handle, so it must be `Send + Sync` for the handles to be `Send`. The
// `Cell<c_int>` slot is only ever mutated on the owning thread (the move-only
// contract); marking `LuaRef` `Sync` is sound under that contract. Handles stay
// `!Sync` via their own `NotSync` markers.
#[cfg(feature = "send")]
unsafe impl Send for LuaRef {}
#[cfg(feature = "send")]
unsafe impl Sync for LuaRef {}

impl LuaRef {
    /// The owning [`Lua`] handle (a fresh borrow sharing the same inner state).
    pub(crate) fn lua(&self) -> Lua {
        Lua {
            inner: self.inner.clone(),
            _not_sync: NOT_SYNC,
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
    pub(crate) fn pop_error(&self, status: c_int) -> Error {
        let state = self.state();
        unsafe {
            // First, see if the error object is one of our *structured* error
            // userdata (raised for scope-destruction errors). If so, recover the
            // original `Error` and wrap it in `CallbackError`, mirroring mlua.
            if let Some(cause) = crate::callback::recover_wrapped_error(state, -1) {
                lua_pop(state, 1);
                return Error::CallbackError {
                    traceback: String::new(),
                    cause: std::sync::Arc::new(cause),
                };
            }
            // Otherwise, fall back to the flat string error path.
            let mut len = 0usize;
            let s = lua_tolstring(state, -1, &mut len);
            let msg = if s.is_null() {
                "<non-string error>".to_string()
            } else {
                let bytes = core::slice::from_raw_parts(s as *const u8, len);
                String::from_utf8_lossy(bytes).into_owned()
            };
            lua_pop(state, 1);
            // `LUA_ERRMEM` (status 4) is an out-of-memory error (the VM set the
            // error object to "not enough memory"); surface it as `MemoryError`
            // so `set_memory_limit` callers can match it, mirroring mlua.
            // `luau_load` reports OOM with a generic non-zero rc but the same
            // "not enough memory" message, so we also detect it by message.
            if status == 4 || msg == "not enough memory" {
                return Error::MemoryError(msg);
            }
            Error::RuntimeError(msg)
        }
    }
}
