//! Luau-specific `Lua` extensions: sandboxing, safeenv, fflags, and the
//! per-VM compiler. Mirrors the Luau-only parts of mlua's `Lua` surface.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::compiler::Compiler;
use crate::error::{Error, Result};
use crate::state::Lua;
use crate::sys::*;
use crate::table::Table;
use crate::thread::Thread;

thread_local! {
    /// Per-VM compiler installed via `Lua::set_compiler`, keyed by global state.
    static VM_COMPILERS: RefCell<HashMap<*mut core::ffi::c_void, Compiler>> =
        RefCell::new(HashMap::new());

    /// Per-VM `catch_rust_panics` option (recorded from `LuaOptions`), keyed by
    /// global state. Currently recorded for parity; see `set_catch_rust_panics`.
    static VM_CATCH_PANICS: RefCell<HashMap<*mut core::ffi::c_void, bool>> =
        RefCell::new(HashMap::new());

    /// Per-VM "is sandboxed" flag, keyed by global state. Mirrors mlua's
    /// `extra.sandboxed`; consulted by `Lua::set_globals`.
    static VM_SANDBOXED: RefCell<HashMap<*mut core::ffi::c_void, bool>> =
        RefCell::new(HashMap::new());
}

/// Registry name under which `sandbox(true)` saves the ORIGINAL globals table so
/// `sandbox(false)` can restore it. Kept in the state's registry (freed on
/// `lua_close`), NOT in a thread-local `Table` handle: a handle owns an
/// `XRc<LuaInner>`, so a thread-local holding it would pin the VM alive forever
/// (the state could never drop) if the caller dropped a still-sandboxed `Lua`.
const SANDBOX_SAVED_GLOBALS_NAME: &str = "__luaur_sandbox_saved_globals";

unsafe fn global_key(state: *mut lua_State) -> *mut core::ffi::c_void {
    unsafe { (*state).global as *mut core::ffi::c_void }
}

/// Drop this VM's compiler / catch-panics / sandboxed-flag entries so they don't
/// leak one slot per state created. Called from `LuaInner::drop`. (These hold no
/// Lua handle, so the state drops normally and this runs; the saved-globals table
/// lives in the registry and is freed with the state on `lua_close`.)
pub(crate) fn clear_vm_state(state: *mut lua_State) {
    let key = unsafe { global_key(state) };
    VM_COMPILERS.with(|m| {
        m.borrow_mut().remove(&key);
    });
    VM_CATCH_PANICS.with(|m| {
        m.borrow_mut().remove(&key);
    });
    VM_SANDBOXED.with(|m| {
        m.borrow_mut().remove(&key);
    });
}

impl Lua {
    /// Enable or disable sandbox mode. Mirrors `mlua::Lua::sandbox`.
    ///
    /// Enabling sets every library table (and the globals table) read-only and
    /// activates `safeenv`, then installs a fresh proxy global table (via
    /// `luaL_sandboxthread`) so that script-level global writes go to a
    /// throwaway table whose `__index` is the original environment. Disabling
    /// restores the original globals table and clears the read-only/safeenv
    /// flags.
    ///
    /// **DEVIATION:** Luau's standard library (as bundled in luaur) does not
    /// register `collectgarbage`; mlua's sandbox test additionally checks that
    /// `collectgarbage` is restricted under the sandbox. That part is not
    /// exercisable here (see `tests/mlua_luau.rs`).
    pub fn sandbox(&self, enabled: bool) -> Result<()> {
        let state = self.state();
        let key = unsafe { global_key(state) };
        VM_SANDBOXED.with(|m| {
            m.borrow_mut().insert(key, enabled);
        });
        unsafe {
            if enabled {
                // Save the ORIGINAL globals table (once) in the registry so we can
                // restore it later. Registry-rooted, not a thread-local handle —
                // see SANDBOX_SAVED_GLOBALS_NAME. `or_insert` semantics: only save
                // if not already saved (a second sandbox(true) keeps the first).
                if self
                    .named_registry_value::<Table>(SANDBOX_SAVED_GLOBALS_NAME)
                    .is_err()
                {
                    let original = self.globals();
                    let _ = self.set_named_registry_value(SANDBOX_SAVED_GLOBALS_NAME, original);
                }
                // Make libraries + base metatables read-only and set safeenv.
                lua_l_sandbox(state);
                // Install the proxy global table for script-level writes.
                lua_l_sandboxthread(state);
            } else {
                // Restore the original globals table (dropping the proxy and any
                // globals written into it), then clear the saved slot.
                if let Ok(orig) = self.named_registry_value::<Table>(SANDBOX_SAVED_GLOBALS_NAME) {
                    orig.push_to_stack();
                    lua_replace(state, LUA_GLOBALSINDEX);
                    // Clear read-only + safeenv on the restored globals so it is
                    // writable again.
                    lua_setreadonly(state, LUA_GLOBALSINDEX, 0);
                    lua_setsafeenv(state, LUA_GLOBALSINDEX, 0);
                    // Also clear read-only on the library tables.
                    self.clear_library_readonly();
                    // Clear the saved slot so a later sandbox(true) re-saves.
                    let _ = self.set_named_registry_value(
                        SANDBOX_SAVED_GLOBALS_NAME,
                        crate::value::Value::Nil,
                    );
                }
            }
        }
        Ok(())
    }

    /// Clear the read-only flag on every library table reachable from the
    /// (restored) globals. Used when leaving sandbox mode.
    fn clear_library_readonly(&self) {
        let globals = self.globals();
        if let Ok(pairs) = globals
            .pairs::<crate::value::Value, crate::value::Value>()
            .collect::<Result<Vec<_>>>()
        {
            for (_, v) in pairs {
                if let crate::value::Value::Table(t) = v {
                    t.set_readonly(false);
                }
            }
        }
    }

    /// Set or clear the `safeenv` flag on the globals table. Mirrors
    /// `mlua::Globals::set_safeenv` applied to the main globals.
    ///
    /// `safeenv` lets the VM fast-path global reads; clearing it forces the slow
    /// path (needed when globals/`__index` may change at runtime).
    pub fn set_safeenv(&self, enabled: bool) {
        let state = self.state();
        unsafe {
            lua_setsafeenv(state, LUA_GLOBALSINDEX, enabled as c_int);
        }
    }

    /// Install a default [`Compiler`] used to compile every chunk loaded by this
    /// VM (unless a chunk overrides it via
    /// [`Chunk::set_compiler`](crate::Chunk::set_compiler)). Mirrors
    /// `mlua::Lua::set_compiler`.
    pub fn set_compiler(&self, compiler: Compiler) {
        let state = self.state();
        let key = unsafe { global_key(state) };
        VM_COMPILERS.with(|m| {
            m.borrow_mut().insert(key, compiler);
        });
    }

    /// Record the `catch_rust_panics` behavioral option for this VM.
    ///
    /// **DEVIATION:** luaur-rt's callback trampoline always catches a Rust panic
    /// and converts it into a catchable Lua error (so the VM is never left
    /// half-unwound). The mlua option that lets a panic propagate as a Rust
    /// unwind across the VM boundary is therefore recorded here but not enforced
    /// — see the deferred `test_panic` in `tests/mlua_core.rs`.
    pub(crate) fn set_catch_rust_panics(&self, enabled: bool) {
        let state = self.state();
        let key = unsafe { global_key(state) };
        VM_CATCH_PANICS.with(|m| {
            m.borrow_mut().insert(key, enabled);
        });
    }

    /// Whether this VM is currently sandboxed (set by [`Lua::sandbox`]). Mirrors
    /// mlua's `extra.sandboxed` flag; consulted by [`Lua::set_globals`].
    pub(crate) fn is_sandboxed(&self) -> bool {
        let state = self.state();
        let key = unsafe { global_key(state) };
        VM_SANDBOXED.with(|m| m.borrow().get(&key).copied().unwrap_or(false))
    }

    /// The VM-default compiler installed via [`Lua::set_compiler`], if any.
    pub(crate) fn vm_compiler(&self) -> Option<Compiler> {
        let state = self.state();
        let key = unsafe { global_key(state) };
        VM_COMPILERS.with(|m| m.borrow().get(&key).cloned())
    }

    /// Set (or clear) the metatable shared by all values of a Luau built-in
    /// type `T`. Mirrors `mlua::Lua::set_type_metatable`.
    ///
    /// Implemented for [`Vector`](crate::Vector), `bool`, [`Number`](f64),
    /// [`LuaString`](crate::LuaString), [`Function`](crate::Function),
    /// [`Thread`](crate::Thread), and
    /// [`LightUserData`](crate::LightUserData). Setting it installs a metatable
    /// in the VM's global per-type metatable slot, so e.g. `v.x`/`v:method`
    /// dispatch through it.
    pub fn set_type_metatable<T: TypeMetatable>(&self, metatable: Option<Table>) {
        T::set_type_metatable(self, metatable);
    }

    /// The metatable shared by all values of a Luau built-in type `T`, if one
    /// has been installed. Mirrors `mlua::Lua::type_metatable`.
    pub fn type_metatable<T: TypeMetatable>(&self) -> Option<Table> {
        T::type_metatable(self)
    }

    /// Set a Luau fast-flag (FFlag) by name. Mirrors `mlua::Lua::set_fflag`.
    ///
    /// **DEVIATION:** luaur's FastFlags are a fixed, compile-time `FFlag` enum
    /// rather than a string-keyed registry, so there is no way to look a flag up
    /// by an arbitrary name. This therefore always reports the name as unknown
    /// (`Err`) — which matches mlua's contract for an unrecognized flag (the
    /// only behavior its `test_fflags` asserts). Known flags are configured at
    /// VM-construction time via `luaur_common::set_all_flags`.
    pub fn set_fflag(name: &str, _enabled: bool) -> Result<()> {
        Err(Error::runtime(format!("fflag '{name}' is not supported")))
    }
}

impl Thread {
    /// Sandbox this coroutine: install a fresh proxy global table on its own
    /// state so global writes inside the coroutine stay local to it. Mirrors
    /// `mlua::Thread::sandbox`.
    pub fn sandbox(&self) -> Result<()> {
        let co = self.thread_state;
        unsafe {
            lua_l_sandboxthread(co);
        }
        Ok(())
    }
}

/// Luau built-in types that have a shared, per-type metatable settable via
/// [`Lua::set_type_metatable`]. Mirrors mlua's sealed `LuauType` trait.
pub trait TypeMetatable: private::Sealed {
    /// Push a representative value of this type onto the stack (so the VM's
    /// `lua_setmetatable`/`lua_getmetatable` operate on the type's global slot).
    #[doc(hidden)]
    unsafe fn push_representative(state: *mut lua_State);

    /// Install (or clear) the shared metatable for this type.
    fn set_type_metatable(lua: &Lua, metatable: Option<Table>) {
        let state = lua.state();
        unsafe {
            Self::push_representative(state);
            match metatable {
                Some(mt) => mt.push_to_stack(),
                None => crate::sys::lua_pushnil(state),
            }
            // For a non-table/non-userdata value, `lua_setmetatable` stores the
            // metatable in the VM's global per-type slot (`g->mt[type]`).
            crate::sys::lua_setmetatable(state, -2);
            // Pop the representative value left on the stack.
            crate::sys::lua_pop(state, 1);
        }
    }

    /// The shared metatable for this type, if installed.
    fn type_metatable(lua: &Lua) -> Option<Table> {
        let state = lua.state();
        unsafe {
            Self::push_representative(state);
            let has = crate::sys::lua_getmetatable(state, -1);
            if has == 0 {
                // No metatable: pop the representative value.
                crate::sys::lua_pop(state, 1);
                return None;
            }
            // stack: [value, metatable]
            let mt = Table::from_ref(lua.pop_ref());
            crate::sys::lua_pop(state, 1); // pop the representative value
            Some(mt)
        }
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for crate::vector::Vector {}
    impl Sealed for bool {}
    impl Sealed for f64 {}
    impl Sealed for crate::string::LuaString {}
    impl Sealed for crate::function::Function {}
    impl Sealed for crate::thread::Thread {}
    impl Sealed for crate::light_userdata::LightUserData {}
}

impl TypeMetatable for crate::vector::Vector {
    unsafe fn push_representative(state: *mut lua_State) {
        unsafe {
            crate::sys::lua_pushvector_lua_state_f32_f32_f32_f32(state, 0.0, 0.0, 0.0, 0.0);
        }
    }
}

impl TypeMetatable for bool {
    unsafe fn push_representative(state: *mut lua_State) {
        unsafe { crate::sys::lua_pushboolean(state, 0) }
    }
}

impl TypeMetatable for f64 {
    unsafe fn push_representative(state: *mut lua_State) {
        unsafe { crate::sys::lua_pushnumber(state, 0.0) }
    }
}

impl TypeMetatable for crate::string::LuaString {
    unsafe fn push_representative(state: *mut lua_State) {
        unsafe {
            let s = c"";
            crate::sys::lua_pushlstring(state, s.as_ptr() as *const c_char, 0);
        }
    }
}

impl TypeMetatable for crate::function::Function {
    unsafe fn push_representative(state: *mut lua_State) {
        // Push a throwaway C function so `lua_setmetatable` targets the global
        // function-type slot.
        unsafe {
            crate::sys::lua_pushcclosurek(state, Some(noop_cfn), c"".as_ptr(), 0, None);
        }
    }
}

impl TypeMetatable for crate::thread::Thread {
    unsafe fn push_representative(state: *mut lua_State) {
        // A fresh thread targets the global thread-type slot.
        unsafe {
            crate::sys::lua_newthread(state);
        }
    }
}

impl TypeMetatable for crate::light_userdata::LightUserData {
    unsafe fn push_representative(state: *mut lua_State) {
        crate::sys::lua_pushlightuserdatatagged(state, core::ptr::null_mut(), 0);
    }
}

/// A do-nothing C function used as the representative value for the
/// function-type metatable slot.
unsafe fn noop_cfn(_state: *mut lua_State) -> c_int {
    0
}

#[cfg(test)]
pub(crate) fn vm_compilers_len() -> usize {
    VM_COMPILERS.with(|m| m.borrow().len())
}
