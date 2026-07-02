//! Luau interrupt support. Mirrors `mlua::Lua::set_interrupt` / `VmState`.
//!
//! Luau's VM calls a single global `interrupt` callback at safepoints (loop
//! back-edges, calls/returns, GC). mlua exposes this as `Lua::set_interrupt`,
//! taking a Rust closure that returns a [`VmState`] telling the VM whether to
//! continue or to **yield** the current coroutine.
//!
//! luaur's `lua_callbacks().interrupt` is a plain C function pointer, so we
//! install a fixed trampoline ([`interrupt_trampoline`]) and keep the Rust
//! closure in a thread-local keyed by the VM's *global* pointer (shared by all
//! threads of one `Lua`). The trampoline looks up the closure, runs it with a
//! borrowed [`Lua`], and:
//!
//! * `Ok(VmState::Continue)`  — returns normally; the VM keeps executing.
//! * `Ok(VmState::Yield)`     — calls `lua_break`, which sets the running
//!   thread's status so the VM unwinds back to `lua_resume` (a *yield* at a
//!   yieldable point; ignored otherwise, exactly like upstream Luau).
//! * `Err(e)`                 — raises `e` as a Lua error via `lua_error`.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::state::Lua;
use crate::sys::*;

/// The action an interrupt callback asks the VM to take. Mirrors
/// `mlua::VmState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmState {
    /// Keep executing.
    Continue,
    /// Yield the currently running coroutine (no-op at a non-yieldable point).
    Yield,
}

type InterruptFn = Box<dyn Fn(&Lua) -> Result<VmState> + 'static>;

thread_local! {
    /// Per-VM interrupt closure, keyed by the `global_State` pointer (stable for
    /// the lifetime of the VM and shared by all of its threads).
    static INTERRUPTS: RefCell<HashMap<*mut core::ffi::c_void, InterruptFn>> =
        RefCell::new(HashMap::new());
}

/// The `global_State` pointer for `state` — the per-VM key shared by all
/// threads of one `Lua`.
unsafe fn vm_key(state: *mut lua_State) -> *mut core::ffi::c_void {
    unsafe { (*state).global as *mut core::ffi::c_void }
}

impl Lua {
    /// Install an interrupt callback. Mirrors `mlua::Lua::set_interrupt`.
    ///
    /// The callback runs at VM safepoints; returning [`VmState::Yield`] yields
    /// the running coroutine, and returning `Err` raises a Lua error.
    pub fn set_interrupt<F>(&self, callback: F)
    where
        F: Fn(&Lua) -> Result<VmState> + crate::sync::MaybeSend + 'static,
    {
        let state = self.state();
        unsafe {
            let key = vm_key(state);
            INTERRUPTS.with(|m| {
                m.borrow_mut().insert(key, Box::new(callback));
            });
            let cb = lua_callbacks(state);
            (*cb).interrupt = Some(interrupt_trampoline);
        }
    }

    /// Remove a previously installed interrupt callback. Mirrors
    /// `mlua::Lua::remove_interrupt`.
    pub fn remove_interrupt(&self) {
        let state = self.state();
        unsafe {
            let key = vm_key(state);
            INTERRUPTS.with(|m| {
                m.borrow_mut().remove(&key);
            });
            let cb = lua_callbacks(state);
            (*cb).interrupt = None;
        }
    }
}

/// Drop this VM's interrupt closure. Called from `LuaInner::drop` so the closure
/// (and anything it captured) is released and the per-VM map entry does not leak
/// one slot per state created. (If a closure captured Lua handles it would pin
/// the VM and this never runs — but the common case captures non-Lua state.)
pub(crate) fn clear_interrupt(state: *mut lua_State) {
    let key = unsafe { vm_key(state) };
    INTERRUPTS.with(|m| {
        m.borrow_mut().remove(&key);
    });
}

/// The fixed C trampoline installed as `lua_callbacks().interrupt`.
///
/// `gc` is non-negative only for GC interrupts; mlua ignores GC interrupts in
/// the user callback path, and so do we (return immediately) so the user
/// closure only sees real instruction safepoints.
unsafe extern "C-unwind" fn interrupt_trampoline(state: *mut lua_State, gc: c_int) {
    if gc >= 0 {
        // GC step interrupt — not surfaced to the user callback.
        return;
    }
    let key = unsafe { vm_key(state) };
    // Take the closure out of the map for the duration of the call so a
    // re-entrant `set_interrupt` from inside the callback can't alias the
    // borrow. Put it back afterwards (unless the callback replaced it).
    let cb = INTERRUPTS.with(|m| m.borrow_mut().remove(&key));
    let Some(cb) = cb else { return };

    let lua = unsafe { Lua::from_borrowed(state) };
    let result = cb(&lua);

    // Restore the closure if the callback didn't install a new one.
    INTERRUPTS.with(|m| {
        let mut map = m.borrow_mut();
        map.entry(key).or_insert(cb);
    });

    match result {
        Ok(VmState::Continue) => {}
        Ok(VmState::Yield) => unsafe {
            // Request a yield — but only at a yieldable point. Inside a
            // metamethod / C-call boundary Luau's `lua_break` would raise
            // "attempt to break across metamethod/C-call boundary"; upstream
            // (and mlua) silently ignore the yield request there, so we gate it
            // on `lua_isyieldable` and otherwise just continue.
            if lua_isyieldable(state) != 0 {
                let _ = luaur_vm::functions::lua_break::lua_break(state);
            }
        },
        Err(e) => unsafe {
            // Raise the error as a Lua error. Push the message and longjmp.
            raise_error(state, &e);
        },
    }
}

/// Push `e`'s message as a string error object and `lua_error` it (does not
/// return).
unsafe fn raise_error(state: *mut lua_State, e: &Error) -> ! {
    // Use the bare message for a runtime error (so it round-trips back through
    // `pop_error` as `RuntimeError(msg)` without a doubled "runtime error: "
    // prefix); fall back to the full Display for other error kinds.
    let msg = match e {
        Error::RuntimeError(m) => m.clone(),
        other => other.to_string(),
    };
    unsafe {
        // The interrupt fires at an arbitrary VM safepoint where `L->top` may be
        // flush against the call-info top; make room before pushing so the
        // `api_incr_top` stack invariant in `lua_pushlstring` holds.
        lua_rawcheckstack(state, 1);
        lua_pushlstring(state, msg.as_ptr() as *const c_char, msg.len());
        lua_error(state)
    }
}

#[cfg(test)]
pub(crate) fn interrupts_len() -> usize {
    INTERRUPTS.with(|m| m.borrow().len())
}
