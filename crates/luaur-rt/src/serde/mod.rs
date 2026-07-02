//! (De)serialization support using [`serde`].
//!
//! This module mirrors `mlua`'s `serde` feature: it adds the [`LuaSerdeExt`]
//! trait to [`Lua`], a serde [`Serializer`] that builds Lua [`Value`]s, and a
//! [`Deserializer`] that reads them. It is gated behind the `serde` cargo
//! feature; with the feature off the rest of the crate is byte-for-byte
//! unchanged.
//!
//! ## Luau deviations from mlua's Lua-version-specific behavior
//!
//! - **Numbers are `f64`.** Luau has no distinct integer subtype at the VM
//!   level; luaur-rt reconstructs [`Value::Integer`] vs [`Value::Number`] from
//!   whether an `f64` is an exact, in-range whole number (see
//!   [`crate::Value`]). Serialization therefore treats an integral `f64` the
//!   same as an integer — matching mlua's *observable* JSON output for the
//!   values under test.
//! - **`null` sentinel.** mlua encodes a serde "none"/JSON `null` using a
//!   `LightUserData(NULL)` value. luaur-rt's [`Value`] has no `LightUserData`
//!   variant, so [`LuaSerdeExt::null`] instead returns a dedicated, per-`Lua`
//!   **sentinel table** (a unique empty table cached in the state). Serde
//!   recognizes this exact table by pointer identity and treats it as
//!   `null`/`None`, reproducing mlua's behavior at the API level.

use std::cell::RefCell;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::ser::Serialize;

use crate::error::Result;
use crate::state::Lua;
use crate::sys::lua_State;
use crate::table::Table;
use crate::value::Value;

pub mod de;
pub mod ser;
mod value_serialize;

pub use de::{Deserializer, Options as DeserializeOptions};
pub use ser::{Options as SerializeOptions, Serializer};
pub use value_serialize::{SerializableTable, SerializableValue};

// ---------------------------------------------------------------------------
// Per-state sentinels (the `null` table and the array metatable).
//
// mlua keeps these in the Lua registry. luaur-rt's `Value` has no
// `LightUserData`, so the `null` value is modelled as a dedicated, unique empty
// table per `Lua`; the array metatable is likewise a unique table per `Lua`.
//
// The tables are ROOTED IN THE LUA REGISTRY (named registry keys) — part of the
// state, freed when it closes. A thread-local keyed by the raw state pointer
// caches only their raw POINTERS for the identity check; it holds no `Table` or
// registry HANDLE. That is deliberate: a handle owns an `XRc<LuaInner>`, so
// caching one here would keep the whole VM alive forever (the state could never
// drop) and, at thread/process exit, dropping the cached handle into an
// already-freed state aborts ("thread local panicked on drop"). Caching only raw
// pointers breaks that cycle — the user's `Lua` closes normally, and
// `clear_sentinels` (from `LuaInner::drop`) evicts the pointer entry. `Lua` is
// `!Send`/`!Sync` (single-threaded), so the thread-local is sound. Compiled only
// under the `serde` feature.
// ---------------------------------------------------------------------------

/// Registry names under which a state's sentinel tables are rooted. Each Lua
/// state has its own registry, so these do not collide across states.
const NULL_REGISTRY_NAME: &str = "__luaur_serde_null_sentinel";
const ARRAY_MT_REGISTRY_NAME: &str = "__luaur_serde_array_metatable";

/// Raw POINTERS of a state's sentinel tables, cached for the identity check.
/// Not `Table` handles: a handle owns an `XRc<LuaInner>`, so caching one here
/// would pin the whole VM alive forever and abort at thread-local teardown (see
/// the module note). Raw pointers are `Copy` and own nothing.
#[derive(Clone, Copy)]
struct SentinelPtrs {
    null: *const core::ffi::c_void,
    array_metatable: *const core::ffi::c_void,
}

thread_local! {
    static SENTINELS: RefCell<HashMap<*mut lua_State, SentinelPtrs>> =
        RefCell::new(HashMap::new());
}

/// Ensure both sentinel tables exist for `lua`: create them, root them in the
/// registry (so they outlive the local handles without pinning the VM), and
/// cache their pointers. Idempotent.
fn ensure_sentinels(lua: &Lua) {
    let key = lua.state();
    if SENTINELS.with(|c| c.borrow().contains_key(&key)) {
        return;
    }
    let null = lua.create_table();
    let array_metatable = build_array_metatable(lua);
    let ptrs = SentinelPtrs {
        null: null.to_pointer(),
        array_metatable: array_metatable.to_pointer(),
    };
    // Root the tables in the registry (freed with the state on `lua_close`); the
    // thread-local keeps only their pointers.
    let _ = lua.set_named_registry_value(NULL_REGISTRY_NAME, null);
    let _ = lua.set_named_registry_value(ARRAY_MT_REGISTRY_NAME, array_metatable);
    SENTINELS.with(|c| {
        c.borrow_mut().insert(key, ptrs);
    });
}

/// Returns the per-`Lua` `null` sentinel table, creating it on first use. The
/// returned handle is transient (fetched from the registry); it drops after the
/// caller, so nothing permanent pins the VM.
pub(crate) fn null_table(lua: &Lua) -> Table {
    ensure_sentinels(lua);
    lua.named_registry_value::<Table>(NULL_REGISTRY_NAME)
        .expect("serde null sentinel is rooted in the registry")
}

/// Returns the per-`Lua` array metatable, creating it on first use.
pub(crate) fn array_metatable_table(lua: &Lua) -> Table {
    ensure_sentinels(lua);
    lua.named_registry_value::<Table>(ARRAY_MT_REGISTRY_NAME)
        .expect("serde array metatable is rooted in the registry")
}

/// Build the array metatable: a fresh table with `__metatable = false` so Lua
/// code cannot read or replace it (mirrors mlua, which protects the metatable
/// the same way).
fn build_array_metatable(lua: &Lua) -> Table {
    let mt = lua.create_table();
    // Best-effort: if this ever failed we'd still have a usable (if
    // unprotected) metatable, so ignore the (impossible here) error.
    let _ = mt.raw_set("__metatable", false);
    mt
}

/// Whether `value` is the `null` sentinel for its owning `Lua` (compared by
/// table pointer identity).
pub(crate) fn is_null(value: &Value) -> bool {
    if let Value::Table(t) = value {
        let key = t.lua().state();
        return SENTINELS.with(|cell| {
            cell.borrow()
                .get(&key)
                .map(|s| s.null == t.to_pointer())
                .unwrap_or(false)
        });
    }
    false
}

/// Whether `table` carries the array metatable (compared by pointer identity).
pub(crate) fn has_array_metatable(table: &Table) -> bool {
    let key = table.lua().state();
    let array_ptr = SENTINELS.with(|cell| cell.borrow().get(&key).map(|s| s.array_metatable));
    match array_ptr {
        Some(ptr) => table
            .metatable()
            .map(|mt| mt.to_pointer() == ptr)
            .unwrap_or(false),
        None => false,
    }
}

/// Evict this state's cached sentinel POINTERS. Called from `LuaInner::drop`; the
/// sentinel tables themselves are freed with the state's registry on `lua_close`.
/// Without this the (now stale) pointer entry would leak one slot per state.
pub(crate) fn clear_sentinels(state: *mut lua_State) {
    SENTINELS.with(|c| {
        c.borrow_mut().remove(&state);
    });
}

/// Trait for serializing/deserializing Lua values using Serde. Mirrors
/// `mlua::LuaSerdeExt`.
pub trait LuaSerdeExt {
    /// A special value used to encode/decode optional (none) values.
    ///
    /// In luaur-rt this is a dedicated, per-`Lua` sentinel [`Table`] (see the
    /// module docs); mlua uses a `LightUserData(NULL)`. The observable behavior
    /// — `null` round-trips to/from serde `None`/JSON `null` — is the same.
    fn null(&self) -> Value;

    /// A metatable attachable to a Lua table to systematically encode it as an
    /// array (instead of a map). The encoded array contains only the sequence
    /// part of the table, with the same length as the `#` operator.
    fn array_metatable(&self) -> Table;

    /// Converts `T` into a [`Value`] instance.
    fn to_value<T: Serialize + ?Sized>(&self, t: &T) -> Result<Value>;

    /// Converts `T` into a [`Value`] instance with options.
    fn to_value_with<T>(&self, t: &T, options: ser::Options) -> Result<Value>
    where
        T: Serialize + ?Sized;

    /// Deserializes a [`Value`] into any serde-deserializable object.
    #[allow(clippy::wrong_self_convention)]
    fn from_value<T: DeserializeOwned>(&self, value: Value) -> Result<T>;

    /// Deserializes a [`Value`] into any serde-deserializable object with
    /// options.
    #[allow(clippy::wrong_self_convention)]
    fn from_value_with<T: DeserializeOwned>(&self, value: Value, options: de::Options)
        -> Result<T>;
}

impl LuaSerdeExt for Lua {
    fn null(&self) -> Value {
        Value::Table(null_table(self))
    }

    fn array_metatable(&self) -> Table {
        array_metatable_table(self)
    }

    fn to_value<T>(&self, t: &T) -> Result<Value>
    where
        T: Serialize + ?Sized,
    {
        t.serialize(ser::Serializer::new(self))
    }

    fn to_value_with<T>(&self, t: &T, options: ser::Options) -> Result<Value>
    where
        T: Serialize + ?Sized,
    {
        t.serialize(ser::Serializer::new_with_options(self, options))
    }

    fn from_value<T>(&self, value: Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(de::Deserializer::new(value))
    }

    fn from_value_with<T>(&self, value: Value, options: de::Options) -> Result<T>
    where
        T: DeserializeOwned,
    {
        T::deserialize(de::Deserializer::new_with_options(value, options))
    }
}

#[cfg(test)]
pub(crate) fn sentinels_len() -> usize {
    SENTINELS.with(|m| m.borrow().len())
}
