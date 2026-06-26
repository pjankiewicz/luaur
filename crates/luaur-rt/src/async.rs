//! The Rust-`Future` ⟷ Lua-coroutine bridge (the `async` feature).
//!
//! This module mirrors mlua's async design: a Rust async function is exposed to
//! Lua as an ordinary Lua **closure** that, when called, drives a boxed
//! `Future` to completion by repeatedly polling it and **yielding** the
//! coroutine while the future is `Pending`. A Rust-side driver
//! ([`AsyncThread`](crate::thread::AsyncThread)) resumes that coroutine and,
//! when it yields the internal "pending" marker, returns `Poll::Pending` to the
//! caller's executor (after registering the executor's `Waker`).
//!
//! ## The poller closure
//!
//! [`create_async_callback`] builds, for each async function, a small Lua
//! closure loaded with a private environment exposing four helpers:
//!
//! * `get_future(...)` — a C closure that, given the call arguments, invokes the
//!   user's Rust async fn, boxes the returned `Future`, stashes it in a userdata
//!   and returns that userdata. (One future per call.)
//! * `poll(future, ...)` — a C closure that polls the stashed future once with
//!   the current [`Waker`] and reports the outcome to the Lua loop.
//! * `yield` — `coroutine.yield`.
//! * `unpack` — spreads a results table back onto the stack.
//!
//! The loop body is byte-for-byte the same control flow as mlua's poller (see
//! the embedded source in [`POLLER_SOURCE`]): poll once; on `Ready` return the
//! results; on `Pending` `yield` the pending marker and poll again on the next
//! resume. Intermediate values produced by [`Lua::yield_with`](crate::Lua) ride
//! through the `yield`/`poll` exchange.
//!
//! ## Markers
//!
//! Three process-unique light-userdata sentinels distinguish poll outcomes
//! across the Lua boundary (mirroring `mlua::Lua::poll_pending` etc.):
//!
//! * **pending** — yielded by the loop when the future is `Pending`; the driver
//!   recognises it and returns `Poll::Pending`.
//! * **yield** — pushed by [`Lua::yield_with`] to mark a value-carrying yield.
//! * **terminate** — passed *into* the loop by the driver when it is dropped
//!   while the coroutine is suspended, so the loop drops the future and parks.
//!
//! ## Soundness
//!
//! * The boxed future lives inside a Lua **userdata** with a destructor, so it
//!   is owned by the coroutine for exactly as long as the coroutine is alive;
//!   when the coroutine (or the whole `Lua`) is collected, the userdata
//!   destructor drops the future, ending its borrows. It is never polled after
//!   the coroutine is dead because polling only happens from inside a live
//!   resume of that coroutine.
//! * The [`Waker`] is borrowed for the duration of a single resume only, via a
//!   thread-local guard ([`WakerGuard`]) that restores the previous waker on
//!   drop — so a future polled during a resume always sees a valid waker, and no
//!   waker reference outlives the `&Context` it came from.
//! * `Lua` is `!Send`/`!Sync` (it is `Rc`-based), so the thread-local waker slot
//!   is never raced.

#![cfg(feature = "async")]

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use crate::error::{Error, Result};
use crate::function::Function;
use crate::multi::MultiValue;
use crate::state::Lua;
use crate::sync::MaybeSend;
use crate::sys::*;
use crate::table::Table;
use crate::thread::{AsyncResume, Thread};
use crate::traits::{FromLuaMulti, IntoLuaMulti};

/// A pinned, boxed future of (already `MultiValue`-converted) async-callback
/// results. Under the `send` feature it is additionally `Send` so the future
/// can be stashed inside the (movable) VM and the whole VM stay `Send`; without
/// the feature it is a plain local future, byte-identical to before. Mirrors how
/// [`crate::callback::BoxedCallback`] gates its `Send` bound (and mlua's
/// `BoxFuture`).
#[cfg(feature = "send")]
pub(crate) type LocalResultFuture = Pin<Box<dyn Future<Output = Result<MultiValue>> + Send>>;
/// See the `send`-gated variant above.
#[cfg(not(feature = "send"))]
pub(crate) type LocalResultFuture = Pin<Box<dyn Future<Output = Result<MultiValue>>>>;

/// The type-erased async callback: given the calling `Lua` and the call
/// arguments, it produces a pinned, boxed future of the (already
/// `MultiValue`-converted) results. `Send` under the `send` feature (the box
/// and its captured environment), matching [`crate::callback::BoxedCallback`].
#[cfg(feature = "send")]
pub(crate) type AsyncCallback = Box<dyn Fn(Lua, MultiValue) -> LocalResultFuture + Send>;
/// See the `send`-gated variant above.
#[cfg(not(feature = "send"))]
pub(crate) type AsyncCallback = Box<dyn Fn(Lua, MultiValue) -> LocalResultFuture>;

// ---------------------------------------------------------------------------
// Poll markers (process-unique light-userdata sentinels)
// ---------------------------------------------------------------------------

// We use the address of a `static` byte as a process-unique pointer value. The
// pointer is only ever *compared*, never dereferenced, so it is always sound.

static PENDING_MARK: u8 = 0;
static YIELD_MARK: u8 = 0;
static TERMINATE_MARK: u8 = 0;

/// The "future is pending" marker pointer.
#[inline]
pub(crate) fn poll_pending() -> *mut c_void {
    &PENDING_MARK as *const u8 as *mut c_void
}

/// The "this yield carries values" marker pointer (used by `yield_with`).
#[inline]
pub(crate) fn poll_yield() -> *mut c_void {
    &YIELD_MARK as *const u8 as *mut c_void
}

/// The "drop the future and park" marker pointer (sent in on driver drop).
#[inline]
pub(crate) fn poll_terminate() -> *mut c_void {
    &TERMINATE_MARK as *const u8 as *mut c_void
}

// ---------------------------------------------------------------------------
// Per-VM async state: the active waker + the implicit-thread ownership map
// ---------------------------------------------------------------------------
//
// mlua keeps both the current `Waker` and the `thread_ownership_map` in the
// per-`Lua` `extra` block, so they are reachable from any coroutine state of the
// VM and travel with the VM when it is moved across threads under `send`.
// luaur-rt's `Lua` has no such block, so we keep them in a process-wide table
// keyed by the VM's **global-state pointer** — the same VM-identity key the
// `app_data` store uses (`(*state).global`), reachable from any of the VM's
// coroutine states.
//
// * Under `send` the table is a real global `Mutex` (NOT a thread-local): the
//   VM can be created on one thread and driven on another, so a thread-local
//   would be left behind on the origin thread (the soundness gap that used to
//   force `send` and `async` to be mutually exclusive). Different VMs may be
//   driven on different threads concurrently, so the table itself needs a lock;
//   per-VM access stays serialized by the `send` contract. The stored `Waker`
//   is `Send + Sync`; coroutine-state pointers are stored as `usize` (compared
//   only, and cast back to `*mut lua_State` solely to push the owner thread).
// * Without `send` it is a thread-local (single-threaded, zero-cost), matching
//   the original behavior plus per-VM keying.

use std::collections::HashMap;

/// Per-VM async state: the waker installed for the current resume and the
/// implicit-thread ownership map (`co_state addr -> owner_state addr`).
#[derive(Default)]
struct AsyncVmState {
    waker: Option<Waker>,
    ownership: HashMap<usize, usize>,
}

/// The VM-identity key: the global-state pointer as an integer (process-unique,
/// stable for the VM's lifetime, `Send`). Mirrors `app_data`'s `vm_key`.
#[inline]
unsafe fn vm_key(state: *mut lua_State) -> usize {
    unsafe { (*state).global as usize }
}

#[cfg(feature = "send")]
mod async_store {
    use super::AsyncVmState;
    use std::collections::HashMap;
    use std::sync::{LazyLock, Mutex};

    static STORE: LazyLock<Mutex<HashMap<usize, AsyncVmState>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    /// Run `f` with exclusive access to the global async-state table.
    pub(super) fn with<R>(f: impl FnOnce(&mut HashMap<usize, AsyncVmState>) -> R) -> R {
        // Recover from poisoning: a panic mid-update leaves the map usable.
        let mut guard = STORE.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut guard)
    }
}

#[cfg(not(feature = "send"))]
mod async_store {
    use super::AsyncVmState;
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static STORE: RefCell<HashMap<usize, AsyncVmState>> = RefCell::new(HashMap::new());
    }

    pub(super) fn with<R>(f: impl FnOnce(&mut HashMap<usize, AsyncVmState>) -> R) -> R {
        STORE.with(|s| f(&mut s.borrow_mut()))
    }
}

/// Install `waker` as the current waker for the duration of a resume, restoring
/// the previous one on drop. Returned by [`set_current_waker`]. Nested async
/// calls push/pop via the guard.
pub(crate) struct WakerGuard {
    key: usize,
    prev: Option<Waker>,
}

impl Drop for WakerGuard {
    fn drop(&mut self) {
        let key = self.key;
        let prev = self.prev.take();
        async_store::with(|m| {
            if let Some(s) = m.get_mut(&key) {
                s.waker = prev;
            }
        });
    }
}

/// Set the current waker for `state`'s VM, returning a guard that restores the
/// previous one. Keyed by the VM global state, so it is found again from any
/// coroutine state during the resume.
pub(crate) fn set_current_waker(state: *mut lua_State, waker: Waker) -> WakerGuard {
    let key = unsafe { vm_key(state) };
    let prev = async_store::with(|m| m.entry(key).or_default().waker.replace(waker));
    WakerGuard { key, prev }
}

/// Register `co_state` as an implicit thread owned (transitively) by `owner`.
pub(crate) fn register_implicit_thread(co_state: *mut lua_State, owner: *mut lua_State) {
    let key = unsafe { vm_key(co_state) };
    let owner = owner as usize;
    async_store::with(|m| {
        let s = m.entry(key).or_default();
        // Chain to the root owner if `owner` is itself implicit.
        let root = s.ownership.get(&owner).copied().unwrap_or(owner);
        s.ownership.insert(co_state as usize, root);
    });
}

/// Forget the implicit-thread registration for `co_state` (on driver drop).
pub(crate) fn unregister_implicit_thread(co_state: *mut lua_State) {
    let key = unsafe { vm_key(co_state) };
    async_store::with(|m| {
        if let Some(s) = m.get_mut(&key) {
            s.ownership.remove(&(co_state as usize));
        }
    });
}

/// The owner state for `state`, if `state` is a registered implicit thread.
pub(crate) fn implicit_thread_owner(state: *mut lua_State) -> Option<*mut lua_State> {
    let key = unsafe { vm_key(state) };
    async_store::with(|m| {
        m.get(&key)
            .and_then(|s| s.ownership.get(&(state as usize)).copied())
            .map(|p| p as *mut lua_State)
    })
}

/// Drop this VM's entire async-state entry. Called from `LuaInner::drop` (the
/// global state is still valid there), mirroring `app_data::clear_app_data`.
pub(crate) fn clear_async_state(state: *mut lua_State) {
    let key = unsafe { vm_key(state) };
    async_store::with(|m| {
        m.remove(&key);
    });
}

/// A clone of the current waker for `state`'s VM, or a no-op waker if none is
/// installed (e.g. the async function was resumed synchronously via
/// `Thread::resume`, matching mlua's "noop waker outside an executor" behavior).
fn current_waker(state: *mut lua_State) -> Waker {
    let key = unsafe { vm_key(state) };
    async_store::with(|m| m.get(&key).and_then(|s| s.waker.clone())).unwrap_or_else(noop_waker)
}

/// A waker that does nothing when woken.
fn noop_waker() -> Waker {
    use std::task::{RawWaker, RawWakerVTable};
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE), // clone
        |_| {},                                       // wake
        |_| {},                                       // wake_by_ref
        |_| {},                                       // drop
    );
    // SAFETY: the vtable never dereferences the (null) data pointer.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

// ---------------------------------------------------------------------------
// The boxed-future userdata stashed by `get_future` and polled by `poll`
// ---------------------------------------------------------------------------

/// The userdata holding the in-flight future for one async call. `data` is
/// `None` once the future has completed or been terminated.
struct AsyncPollUpvalue {
    data: Option<LocalResultFuture>,
}

/// Destructor for the [`AsyncPollUpvalue`] userdata: drops the boxed future
/// (ending any borrows its captured environment holds).
unsafe extern "C" fn poll_upvalue_dtor(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut AsyncPollUpvalue) };
    }
}

/// The userdata holding the type-erased [`AsyncCallback`] (upvalue of the
/// `get_future` C closure).
struct AsyncCallbackUpvalue {
    callback: AsyncCallback,
}

/// Destructor for the [`AsyncCallbackUpvalue`] userdata.
unsafe extern "C" fn callback_upvalue_dtor(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut AsyncCallbackUpvalue) };
    }
}

// ---------------------------------------------------------------------------
// The C closures: get_future + poll
// ---------------------------------------------------------------------------

/// `get_future(...)`: invoke the user's async callback with the call args, box
/// the future, and return it wrapped in an [`AsyncPollUpvalue`] userdata.
unsafe fn get_future_c(state: *mut lua_State) -> c_int {
    unsafe {
        // Recover the callback from upvalue 1.
        let ud = lua_touserdata(state, lua_upvalueindex(1));
        if ud.is_null() {
            return raise(state, "luaur-rt: missing async callback upvalue");
        }
        let upvalue = &*(ud as *const AsyncCallbackUpvalue);

        let lua = Lua::from_borrowed(state);

        // Collect call arguments (stack 1..=nargs).
        let nargs = lua_gettop(state);
        let mut args = MultiValue::with_capacity(nargs.max(0) as usize);
        for i in 1..=nargs {
            match lua.value_from_stack(i) {
                Ok(v) => args.push_back(v),
                Err(e) => return raise(state, &e.to_string()),
            }
        }

        // Build the future (this runs only the synchronous prologue of the user
        // async fn up to the first await point).
        let fut = (upvalue.callback)(lua.clone(), args);

        // Stash it in a fresh userdata with a dtor and return it.
        let storage = lua_newuserdatadtor(
            state,
            core::mem::size_of::<AsyncPollUpvalue>(),
            Some(poll_upvalue_dtor),
        );
        if storage.is_null() {
            return raise(state, "luaur-rt: failed to allocate async future userdata");
        }
        core::ptr::write(
            storage as *mut AsyncPollUpvalue,
            AsyncPollUpvalue { data: Some(fut) },
        );
        1
    }
}

/// `poll(future, ...)`: poll the stashed future once and report the outcome.
///
/// Return convention (matches mlua's poller loop):
/// * `Ready(n)` results: returns `nres = n` followed by up to 2 result values,
///   or `nres, table` for `n >= 3` (the loop `unpack`s the table).
/// * `Pending` (plain): returns `nil, <pending light-userdata>`.
/// * `Pending` (value-carrying, via `yield_with`): returns
///   `nil, <values-table>, <count>` for the loop to forward through `yield`.
/// * terminate signal received: returns `-1` so the loop parks forever.
unsafe fn poll_c(state: *mut lua_State) -> c_int {
    unsafe {
        // The future userdata is always argument 1.
        let ud = lua_touserdata(state, 1);
        if ud.is_null() {
            return raise(state, "luaur-rt: missing async future argument");
        }
        let future = &mut *(ud as *mut AsyncPollUpvalue);

        let nargs = lua_gettop(state);

        // Terminate signal: `poll(future, <terminate light-userdata>)`.
        if nargs == 2 && lua_tolightuserdata(state, -1) == poll_terminate() {
            future.data.take(); // drop the future
            lua_pushinteger(state, -1);
            return 1;
        }

        let lua = Lua::from_borrowed(state);

        let waker = current_waker(state);
        let mut cx = std::task::Context::from_waker(&waker);

        let poll = match future.data.as_mut() {
            Some(f) => f.as_mut().poll(&mut cx),
            None => return raise_destructed(state),
        };

        use std::task::Poll;
        match poll {
            Poll::Pending => {
                let fut_nvals = lua_gettop(state) - 1; // exclude the future itself
                if fut_nvals >= 3 && lua_tolightuserdata(state, -3) == poll_yield() {
                    // A value-carrying yield from `yield_with`: stack tail is
                    // [yield_marker, values_table, count]. Replace the marker
                    // (at -3) with nil so the loop forwards [nil, table, count].
                    lua_pushnil(state);
                    lua_replace(state, -4);
                    return 3;
                }
                // Plain pending: return [nil, pending_marker].
                lua_pushnil(state);
                lua_pushlightuserdatatagged(state, poll_pending(), 0);
                2
            }
            Poll::Ready(result) => {
                let results = match result {
                    Ok(r) => r,
                    Err(e) => {
                        // The future returned `Err` -> raise it as a Lua error so
                        // it propagates through the coroutine like any other.
                        return raise(state, &e.to_string());
                    }
                };
                let nres = results.len() as c_int;
                if nres < 3 {
                    // Fast path: push count then up to 2 results (the loop reads
                    // them as `res`, `res2`).
                    lua_pushinteger(state, nres);
                    for v in results.iter() {
                        if let Err(e) = lua.push_value(v) {
                            return raise(state, &e.to_string());
                        }
                    }
                    1 + nres
                } else {
                    // Many results: pack into a sequence table; loop `unpack`s it.
                    lua_pushinteger(state, nres);
                    let seq = match lua.create_sequence_from(results) {
                        Ok(t) => t,
                        Err(e) => return raise(state, &e.to_string()),
                    };
                    seq.push_to_stack();
                    2
                }
            }
        }
    }
}

/// `unpack(t, n)`: push `t[1]..t[n]` onto the stack and return `n`.
unsafe fn unpack_c(state: *mut lua_State) -> c_int {
    unsafe {
        let mut isnum: c_int = 0;
        let n = lua_tointegerx(state, 2, &mut isnum as *mut c_int);
        if lua_checkstack(state, n.saturating_add(1)) == 0 {
            return raise(state, "luaur-rt: stack overflow unpacking async results");
        }
        for i in 1..=n {
            lua_rawgeti(state, 1, i);
        }
        n
    }
}

// ---------------------------------------------------------------------------
// Small raise helpers (mirroring callback.rs, kept local to avoid coupling)
// ---------------------------------------------------------------------------

/// Push `msg` and `lua_error` (diverges via the VM longjmp).
unsafe fn raise(state: *mut lua_State, msg: &str) -> c_int {
    unsafe {
        lua_pushlstring(state, msg.as_ptr() as *const c_char, msg.len());
        lua_error(state)
    }
}

/// Raise the structured `CallbackDestructed` error (future polled after drop).
unsafe fn raise_destructed(state: *mut lua_State) -> c_int {
    unsafe { crate::callback::raise_structured_error(state, Error::CallbackDestructed) }
}

// ---------------------------------------------------------------------------
// The Lua poller loop source (identical control flow to mlua's)
// ---------------------------------------------------------------------------

const POLLER_SOURCE: &str = r#"
local poll, yield = poll, yield
local future = get_future(...)
local nres, res, res2 = poll(future)
while true do
    if nres ~= nil then
        if nres == 0 then
            return
        elseif nres == 1 then
            return res
        elseif nres == 2 then
            return res, res2
        elseif nres < 0 then
            yield()
        else
            return unpack(res, nres)
        end
    end

    if res2 == nil then
        nres, res, res2 = poll(future, yield(res))
    elseif res2 == 0 then
        nres, res, res2 = poll(future, yield())
    elseif res2 == 1 then
        nres, res, res2 = poll(future, yield(res))
    else
        nres, res, res2 = poll(future, yield(unpack(res, res2)))
    end
end
"#;

// ---------------------------------------------------------------------------
// Building the async function
// ---------------------------------------------------------------------------

/// Push a fresh C closure whose single upvalue is `userdata` (already on top of
/// the stack). Mirrors the callback trampoline construction.
unsafe fn push_c_closure_with_upvalue(
    state: *mut lua_State,
    f: unsafe fn(*mut lua_State) -> c_int,
    name: &core::ffi::CStr,
) {
    unsafe {
        lua_pushcclosurek(state, Some(f), name.as_ptr(), 1, None);
    }
}

/// Build a [`Function`] that, when called from Lua, drives the given async
/// callback to completion (yielding while pending). This is the core of
/// [`Lua::create_async_function`].
pub(crate) fn create_async_callback(lua: &Lua, callback: AsyncCallback) -> Result<Function> {
    let state = lua.state();

    // 1. Build the `get_future` C closure with the callback as its upvalue.
    let get_future = unsafe {
        let storage = lua_newuserdatadtor(
            state,
            core::mem::size_of::<AsyncCallbackUpvalue>(),
            Some(callback_upvalue_dtor),
        );
        if storage.is_null() {
            return Err(Error::runtime(
                "luaur-rt: failed to allocate async callback userdata",
            ));
        }
        core::ptr::write(
            storage as *mut AsyncCallbackUpvalue,
            AsyncCallbackUpvalue { callback },
        );
        push_c_closure_with_upvalue(state, get_future_c, c"luaur-rt-get-future");
        Function::from_ref(lua.pop_ref())
    };

    // 2. Build the `poll` and `unpack` C closures (no upvalues).
    let poll = unsafe {
        lua_pushcclosurek(state, Some(poll_c), c"luaur-rt-poll".as_ptr(), 0, None);
        Function::from_ref(lua.pop_ref())
    };
    let unpack = unsafe {
        lua_pushcclosurek(state, Some(unpack_c), c"luaur-rt-unpack".as_ptr(), 0, None);
        Function::from_ref(lua.pop_ref())
    };

    // 3. Fetch `coroutine.yield`.
    let coroutine: Table = lua.globals().get("coroutine")?;
    let yield_fn: Function = coroutine.get("yield")?;

    // 4. Assemble the poller's private environment.
    let env = lua.create_table();
    env.set("get_future", get_future)?;
    env.set("poll", poll)?;
    env.set("yield", yield_fn)?;
    env.set("unpack", unpack)?;

    // 5. Load the poller loop with that environment and return it as the async
    //    function.
    lua.load(POLLER_SOURCE)
        .set_name("__luaur_async_poll")
        .set_environment(env)
        .into_function()
}

// ---------------------------------------------------------------------------
// `Lua::yield_with` (cooperative value-carrying yield from inside an async fn)
// ---------------------------------------------------------------------------

impl Lua {
    /// Yield the current async coroutine, returning `args` to the resumer, and
    /// resolve to the values the coroutine is next resumed with.
    ///
    /// Mirrors `mlua::Lua::yield_with`. Only valid inside a function created
    /// with [`Lua::create_async_function`] that is being driven on a coroutine.
    ///
    /// Returns an owning `'static` future (rather than being an `async fn`): the
    /// only step that borrows `self` is the eager argument conversion, so the
    /// future itself owns just a `Lua` clone and borrows nothing. That keeps it
    /// `Send` under the `send` feature — which is required because the future is
    /// stored inside the (movable) VM. An `async fn(&self)` would instead capture
    /// `&self` across the await and demand `Lua: Sync`, which luaur-rt
    /// deliberately is **not** (its move-only, never-shared contract).
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    pub fn yield_with<R: crate::traits::FromLuaMulti + 'static>(
        &self,
        args: impl IntoLuaMulti,
    ) -> impl std::future::Future<Output = Result<R>> + 'static {
        let lua = self.clone();
        let args = args.into_lua_multi(self);
        async move {
            let mut args = Some(args?);
            std::future::poll_fn(move |_cx| {
                use std::task::Poll;
                match args.take() {
                    // First poll: push the yield marker + the values + the count so
                    // `poll_c` recognises a value-carrying yield, then report Pending.
                    Some(values) => {
                        let state = lua.state();
                        unsafe {
                            lua_pushlightuserdatatagged(state, poll_yield(), 0);
                            let count = values.len() as c_int;
                            if count <= 1 {
                                // Single value (or none): push it directly.
                                match values.iter().next() {
                                    Some(v) => {
                                        if lua.push_value(v).is_err() {
                                            return Poll::Ready(Err(Error::runtime(
                                                "luaur-rt: failed to push yield value",
                                            )));
                                        }
                                    }
                                    None => lua_pushnil(state),
                                }
                            } else {
                                // Multiple: pack into a sequence table.
                                match lua.create_sequence_from(values) {
                                    Ok(t) => t.push_to_stack(),
                                    Err(e) => return Poll::Ready(Err(e)),
                                }
                            }
                            lua_pushinteger(state, count);
                        }
                        Poll::Pending
                    }
                    // Second poll (after resume): collect the resume values.
                    //
                    // We are running inside `poll(future, <resume values>)`, so the
                    // coroutine stack is `[future, resume1, resume2, ...]`. The
                    // resume values are at indices 2..=top; index 1 (the future) is
                    // *not* a result and must be left in place for `poll_c`.
                    None => {
                        let state = lua.state();
                        let result = unsafe {
                            let top = lua_gettop(state);
                            let mut results = MultiValue::with_capacity((top.max(1) - 1) as usize);
                            let mut err = None;
                            for i in 2..=top {
                                match lua.value_from_stack(i) {
                                    Ok(v) => results.push_back(v),
                                    Err(e) => {
                                        err = Some(e);
                                        break;
                                    }
                                }
                            }
                            // Drop the resume values, keeping the future at index 1.
                            if top > 1 {
                                lua_settop(state, 1);
                            }
                            match err {
                                Some(e) => Err(e),
                                None => R::from_lua_multi(results, &lua),
                            }
                        };
                        Poll::Ready(result)
                    }
                }
            })
            .await
        }
    }
}

// ---------------------------------------------------------------------------
// LuaNativeAsyncFn: arity-abstracting async closure trait (mirrors mlua)
// ---------------------------------------------------------------------------

/// An async function/closure callable with a tuple of `FromLuaMulti` arguments,
/// abstracting over arity. Mirrors `mlua::LuaNativeAsyncFn`. Lets
/// [`Function::wrap_async`](crate::Function::wrap_async) accept `||`, `|a|`,
/// `|a, b|`, … closures uniformly.
/// A pinned, boxed future of an arbitrary output, `Send` under the `send`
/// feature (so a wrapped async closure's future can live in the movable VM).
#[cfg(feature = "send")]
pub type BoxedAsyncFnFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
/// See the `send`-gated variant above.
#[cfg(not(feature = "send"))]
pub type BoxedAsyncFnFuture<T> = Pin<Box<dyn Future<Output = T>>>;

pub trait LuaNativeAsyncFn<A: FromLuaMulti> {
    /// The (non-`Future`) output type produced by the returned future.
    type Output;

    /// Invoke the closure with the converted args, returning its future.
    fn call(&self, args: A) -> BoxedAsyncFnFuture<Self::Output>;
}

macro_rules! impl_lua_native_async_fn {
    ($($A:ident),*) => {
        impl<FN, $($A,)* Fut, R> LuaNativeAsyncFn<($($A,)*)> for FN
        where
            FN: Fn($($A,)*) -> Fut + MaybeSend + 'static,
            ($($A,)*): FromLuaMulti,
            Fut: Future<Output = R> + MaybeSend + 'static,
        {
            type Output = R;

            #[allow(non_snake_case)]
            fn call(&self, args: ($($A,)*)) -> BoxedAsyncFnFuture<R> {
                let ($($A,)*) = args;
                Box::pin(self($($A,)*))
            }
        }
    };
}

impl_lua_native_async_fn!();
impl_lua_native_async_fn!(A);
impl_lua_native_async_fn!(A, B);
impl_lua_native_async_fn!(A, B, C);
impl_lua_native_async_fn!(A, B, C, D);
impl_lua_native_async_fn!(A, B, C, D, E);
impl_lua_native_async_fn!(A, B, C, D, E, F);
impl_lua_native_async_fn!(A, B, C, D, E, F, G);
impl_lua_native_async_fn!(A, B, C, D, E, F, G, H);

// ---------------------------------------------------------------------------
// WrappedAsync: an `IntoLua` adapter for `Function::wrap_async{,_raw}`
// ---------------------------------------------------------------------------

/// An async closure not yet bound to a [`Lua`]. Becomes a Lua async function
/// (via [`Lua::create_async_function`]) when converted with [`IntoLua`].
///
/// Used to implement [`Function::wrap_async`](crate::Function::wrap_async) /
/// [`Function::wrap_raw_async`](crate::Function::wrap_raw_async), which can be
/// constructed without a `Lua` in hand.
pub struct WrappedAsync<F, A, FR, R> {
    func: F,
    _marker: PhantomData<fn(A) -> (FR, R)>,
}

impl<F, A, FR, R> WrappedAsync<F, A, FR, R>
where
    F: Fn(Lua, A) -> FR + MaybeSend + 'static,
    A: FromLuaMulti,
    FR: Future<Output = Result<R>> + MaybeSend + 'static,
    R: crate::traits::IntoLuaMulti,
{
    pub(crate) fn new(func: F) -> Self {
        WrappedAsync {
            func,
            _marker: PhantomData,
        }
    }
}

impl<F, A, FR, R> crate::traits::IntoLua for WrappedAsync<F, A, FR, R>
where
    F: Fn(Lua, A) -> FR + MaybeSend + 'static,
    A: FromLuaMulti,
    FR: Future<Output = Result<R>> + MaybeSend + 'static,
    R: crate::traits::IntoLuaMulti,
{
    fn into_lua(self, lua: &Lua) -> Result<crate::value::Value> {
        let func = self.func;
        let f = lua.create_async_function(move |lua, a: A| func(lua, a))?;
        Ok(crate::value::Value::Function(f))
    }
}

// ---------------------------------------------------------------------------
// AsyncThread: drives a coroutine as a Rust Future / Stream
// ---------------------------------------------------------------------------

/// A coroutine being driven to completion by a Rust executor.
///
/// Mirrors `mlua::AsyncThread`. Created by
/// [`Thread::into_async`](crate::Thread::into_async),
/// [`Function::call_async`](crate::Function::call_async), and the `*_async`
/// chunk helpers.
///
/// * As a [`Future`] it resumes the coroutine until it finishes, discarding any
///   intermediate `coroutine.yield` values, and resolves to the final return
///   value(s) converted to `R`.
/// * As a [`Stream`](futures_util::stream::Stream) it yields one item per
///   `coroutine.yield`, then ends when the coroutine returns.
///
/// While the underlying async function's future is pending, the coroutine
/// yields the internal pending marker and this `AsyncThread` returns
/// `Poll::Pending`, registering the executor's waker for the next poll.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct AsyncThread<R> {
    thread: Thread,
    /// The args for the *next* resume. `Some` until consumed by the first
    /// resume; subsequent resumes pass an empty `MultiValue`.
    args: Option<MultiValue>,
    /// Set once the coroutine has finished (or errored), so a second poll
    /// returns `CoroutineUnresumable` like mlua.
    done: bool,
    /// Whether the underlying coroutine was created implicitly by `call_async`
    /// (and hence registered in the thread-ownership map / to be unregistered
    /// on drop).
    implicit: bool,
    _ret: PhantomData<fn() -> R>,
}

impl<R> AsyncThread<R> {
    pub(crate) fn new(thread: Thread, args: MultiValue) -> AsyncThread<R> {
        AsyncThread {
            thread,
            args: Some(args),
            done: false,
            implicit: false,
            _ret: PhantomData,
        }
    }

    /// Mark this as an implicit (`call_async`-created) thread, so its
    /// ownership-map entry is cleaned up on drop.
    pub(crate) fn set_implicit(&mut self, implicit: bool) {
        self.implicit = implicit;
    }

    /// Take the resume args (first resume gets the real args, later ones empty).
    fn take_args(&mut self) -> MultiValue {
        self.args.take().unwrap_or_default()
    }
}

impl<R> Drop for AsyncThread<R> {
    fn drop(&mut self) {
        // If the coroutine is still suspended (e.g. the executor dropped us mid
        // future, as in `tokio::time::timeout`), resume it once with the
        // terminate signal so it drops its in-flight future and ends its
        // borrows. Best-effort.
        if !self.done {
            self.thread.terminate_async();
        }
        if self.implicit {
            unregister_implicit_thread(self.thread.state());
        }
    }
}

impl<R: FromLuaMulti> Future for AsyncThread<R> {
    type Output = Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.done {
            return Poll::Ready(Err(Error::CoroutineUnresumable));
        }
        let lua = this.thread.lua();
        let _wg = set_current_waker(lua.state(), cx.waker().clone());

        let args = this.take_args();
        match this.thread.resume_for_async(args) {
            Err(e) => {
                this.done = true;
                Poll::Ready(Err(e))
            }
            Ok(AsyncResume::Pending) => {
                // Future is pending; park until the executor re-polls.
                Poll::Pending
            }
            Ok(AsyncResume::Yielded(_vals)) => {
                // As a Future we discard plain `coroutine.yield` values and keep
                // driving: wake immediately so the executor re-polls and resumes.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Ok(AsyncResume::Returned(vals)) => {
                this.done = true;
                Poll::Ready(R::from_lua_multi(vals, &lua))
            }
        }
    }
}

impl<R: FromLuaMulti> futures_util::stream::Stream for AsyncThread<R> {
    type Item = Result<R>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if this.done {
            return Poll::Ready(None);
        }
        let lua = this.thread.lua();
        let _wg = set_current_waker(lua.state(), cx.waker().clone());

        let args = this.take_args();
        match this.thread.resume_for_async(args) {
            Err(e) => {
                this.done = true;
                Poll::Ready(Some(Err(e)))
            }
            Ok(AsyncResume::Pending) => Poll::Pending,
            Ok(AsyncResume::Yielded(vals)) => {
                // A `coroutine.yield` produces a Stream item.
                Poll::Ready(Some(R::from_lua_multi(vals, &lua)))
            }
            Ok(AsyncResume::Returned(vals)) => {
                // Final return is the last Stream item.
                this.done = true;
                Poll::Ready(Some(R::from_lua_multi(vals, &lua)))
            }
        }
    }
}
