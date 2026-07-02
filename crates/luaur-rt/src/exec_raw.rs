//! [`Lua::exec_raw`] and [`Lua::create_c_function`] — escape hatches to the raw
//! luaur stack machine. Mirror `mlua::Lua::exec_raw` / `create_c_function`.
//!
//! `exec_raw` runs a user closure that manipulates the raw stack **inside a
//! protected call**, so a `lua_error` raised by the closure (which luaur
//! implements as a `panic_any(lua_exception)`) is caught by the VM's own
//! `lua_pcall` and surfaced as an [`Error`], exactly like a normal Lua error.
//! Unlike the [`create_function`](crate::Lua::create_function) trampoline, the
//! `exec_raw` trampoline deliberately does **not** `catch_unwind`: the whole
//! point is to let the VM's protected-call machinery handle the unwind.

use std::cell::Cell;

use crate::error::Result;
use crate::function::Function;
use crate::multi::MultiValue;
use crate::state::Lua;
use crate::sys::*;
use crate::traits::{FromLuaMulti, IntoLuaMulti};

/// The boxed, type-erased raw closure stored in the `exec_raw` trampoline's
/// upvalue userdata. `FnMut`-once: it is taken out and run exactly once.
type RawFn = Box<dyn FnOnce(*mut lua_State)>;

/// Userdata storage for the `exec_raw` closure (a `Cell<Option<..>>` so the
/// trampoline can `take` it).
struct RawFnSlot(Cell<Option<RawFn>>);

/// Destructor: drop the (possibly already-taken) closure box.
unsafe extern "C" fn raw_fn_dtor(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut RawFnSlot) };
    }
}

/// The trampoline for `exec_raw`: recover the boxed closure from upvalue 1 and
/// run it on the calling state. Does NOT `catch_unwind` — a `lua_error` from the
/// closure must propagate to the enclosing `lua_pcall`.
unsafe fn exec_raw_trampoline(state: *mut lua_State) -> c_int {
    unsafe {
        let ud = lua_touserdata(state, lua_upvalueindex(1));
        if ud.is_null() {
            return 0;
        }
        let slot = &*(ud as *const RawFnSlot);
        let f = slot.0.take();
        let base = lua_gettop(state);
        if let Some(f) = f {
            f(state);
        }
        // Everything the closure left above the stack base is a result.
        let top = lua_gettop(state);
        (top - base).max(0)
    }
}

fn exec_raw_trampoline_ptr() -> lua_CFunction {
    Some(exec_raw_trampoline)
}

impl Lua {
    /// Run a closure that manipulates the raw luaur stack, under a protected
    /// call. Mirrors `mlua::Lua::exec_raw`.
    ///
    /// `args` are pushed first (as the function arguments); then `f` runs with
    /// the raw `*mut lua_State`, pushing any results it wants returned. A
    /// `lua_error` raised inside `f` is caught and returned as an [`Error`].
    ///
    /// # Safety
    /// `f` operates on the raw stack with no safety net beyond the protected
    /// call; it must leave the stack in a consistent state (push results, not
    /// underflow). This mirrors `mlua::Lua::exec_raw`'s `unsafe` contract.
    pub unsafe fn exec_raw<R, F>(&self, args: impl IntoLuaMulti, f: F) -> Result<R>
    where
        R: FromLuaMulti,
        F: FnOnce(*mut lua_State),
    {
        let state = self.state();
        let args: MultiValue = args.into_lua_multi(self)?;
        // Erase the closure's lifetime: it runs to completion before this
        // function returns (synchronous protected call), so the closure (and
        // anything it borrows) outlives the call.
        let boxed: RawFn = {
            // SAFETY: `f` is consumed within this synchronous call frame (the
            // protected call below runs it to completion before returning), so
            // the closure — and anything it borrows — outlives the box. The
            // transmute only widens the closure's (non-`'static`) lifetime to
            // `'static`; that `'static` box never escapes this function.
            let f: Box<dyn FnOnce(*mut lua_State) + '_> = Box::new(f);
            unsafe { core::mem::transmute::<Box<dyn FnOnce(*mut lua_State) + '_>, RawFn>(f) }
        };
        unsafe {
            let nargs = args.len() as c_int;
            if lua_checkstack(state, nargs.saturating_add(2)) == 0 {
                return Err(crate::error::Error::runtime("stack overflow in exec_raw"));
            }
            // Allocate the slot userdata and write the closure into it.
            let storage =
                lua_newuserdatadtor(state, core::mem::size_of::<RawFnSlot>(), Some(raw_fn_dtor));
            if storage.is_null() {
                return Err(crate::error::Error::runtime(
                    "exec_raw: failed to allocate closure userdata",
                ));
            }
            core::ptr::write(storage as *mut RawFnSlot, RawFnSlot(Cell::new(Some(boxed))));
            // Wrap it in a C closure (consumes the userdata as upvalue 1).
            lua_pushcclosurek(
                state,
                exec_raw_trampoline_ptr(),
                c"luaur-rt-exec-raw".as_ptr(),
                1,
                None,
            );
            // Push the arguments after the function, then protected-call.
            let base = lua_gettop(state) - 1; // index just below the function
            for v in args.iter() {
                self.push_value(v)?;
            }
            let status = lua_pcall(state, nargs, -1, 0);
            if status != 0 {
                return Err(self.pop_error(status));
            }
            // `value_from_stack` duplicates each reference-typed result onto the
            // stack before popping it; after a LUA_MULTRET call the results can
            // fill the C frame exactly to `ci->top`, so reserve headroom or that
            // push overruns (the `lua_pushvalue` api_incr_top assert). Same class
            // as the fix in `function.rs::call`.
            if lua_checkstack(state, 2) == 0 {
                lua_settop(state, base);
                return Err(crate::error::Error::RuntimeError(
                    "stack overflow: too many return values".to_string(),
                ));
            }
            // Collect results left above `base`.
            let top = lua_gettop(state);
            let nresults = top - base;
            let mut results = MultiValue::with_capacity(nresults.max(0) as usize);
            for i in 0..nresults {
                results.push_back(self.value_from_stack(base + 1 + i)?);
            }
            lua_settop(state, base);
            R::from_lua_multi(results, self)
        }
    }

    /// Wrap a raw luaur `lua_CFunction` as a [`Function`]. Mirrors
    /// `mlua::Lua::create_c_function`.
    ///
    /// **DEVIATION:** luaur's `lua_CFunction` is a plain Rust
    /// `Option<unsafe fn(*mut lua_State) -> c_int>` (luaur is a pure-Rust VM with
    /// no C ABI boundary), not an `extern "C-unwind" fn` as in mlua's FFI build.
    /// The function value is otherwise identical; callers pass a luaur-shaped
    /// `unsafe fn` (see [`ffi::lua_CFunction`](crate::sys::lua_CFunction)).
    ///
    /// # Safety
    /// The supplied function runs with raw access to the `lua_State`; it must
    /// honor the luaur calling convention (consume its arguments, push its
    /// results, return the result count). Mirrors mlua's `unsafe` contract.
    pub unsafe fn create_c_function(&self, func: lua_CFunction) -> Result<Function> {
        let state = self.state();
        unsafe {
            lua_pushcclosurek(state, func, c"luaur-rt-c-function".as_ptr(), 0, None);
            Ok(Function::from_ref(self.pop_ref()))
        }
    }
}
