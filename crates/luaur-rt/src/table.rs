//! The [`Table`] handle. Mirrors `mlua::Table`.

use std::rc::Rc;

use crate::error::Result;
use crate::ffi::*;
use crate::state::{Lua, LuaRef};
use crate::traits::{FromLua, IntoLua};
use crate::value::Value;

/// A handle to a Lua table.
///
/// Mirrors `mlua::Table`. Holds a registry reference keeping the table alive.
#[derive(Clone)]
pub struct Table {
    pub(crate) reference: Rc<LuaRef>,
}

impl Table {
    pub(crate) fn from_ref(reference: LuaRef) -> Table {
        Table {
            reference: Rc::new(reference),
        }
    }

    pub(crate) unsafe fn push_to_stack(&self) {
        self.reference.push();
    }

    /// The owning [`Lua`].
    pub fn lua(&self) -> Lua {
        self.reference.lua()
    }

    /// Set `table[key] = value`, honoring metamethods (`__newindex`).
    ///
    /// Mirrors `mlua::Table::set`. Errors (`RuntimeError`) propagate from a
    /// `__newindex` metamethod that raises.
    pub fn set<K: IntoLua, V: IntoLua>(&self, key: K, value: V) -> Result<()> {
        let lua = self.lua();
        let state = lua.state();
        let k = key.into_lua(&lua)?;
        let v = value.into_lua(&lua)?;
        // Drive the (possibly metamethod-invoking) settable under pcall so a
        // raising `__newindex` (or readonly table) surfaces as `Err`.
        unsafe {
            self.reference.push(); // table
            lua.push_value(&k)?; // key
            lua.push_value(&v)?; // value
            let status = protected_settable(state);
            if status != 0 {
                return Err(lua.pop_error(status));
            }
        }
        Ok(())
    }

    /// Get `table[key]`, honoring metamethods (`__index`), converting the
    /// result to `V`.
    ///
    /// Mirrors `mlua::Table::get`. The value type is the sole explicit type
    /// parameter (key type is inferred), matching mlua's `get::<V>(key)`.
    pub fn get<V: FromLua>(&self, key: impl IntoLua) -> Result<V> {
        let lua = self.lua();
        let state = lua.state();
        let k = key.into_lua(&lua)?;
        let value = unsafe {
            self.reference.push(); // table
            lua.push_value(&k)?; // key
            let status = protected_gettable(state);
            if status != 0 {
                return Err(lua.pop_error(status));
            }
            let v = lua.value_from_stack(-1)?;
            lua_pop(state, 1); // pop the result value
            v
        };
        V::from_lua(value, &lua)
    }

    /// Whether `table[key]` is non-nil.
    ///
    /// Mirrors `mlua::Table::contains_key`.
    pub fn contains_key<K: IntoLua>(&self, key: K) -> Result<bool> {
        let v: Value = self.get(key)?;
        Ok(!v.is_nil())
    }

    /// The border length (`#table`).
    ///
    /// Mirrors `mlua::Table::raw_len` (returns `usize`). luaur's `lua_objlen`
    /// gives the same border-length semantics as `lua_rawlen`.
    pub fn raw_len(&self) -> usize {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let n = lua_objlen(state, -1);
            lua_pop(state, 1);
            n.max(0) as usize
        }
    }

    /// The length (`#table`), honoring a `__len` metamethod.
    ///
    /// Mirrors `mlua::Table::len`. Returns `Err` if a `__len` metamethod
    /// raises. Without a `__len` metamethod this is the raw border length.
    pub fn len(&self) -> Result<usize> {
        let lua = self.lua();
        // Fast path: no metatable -> raw border length (no metamethod possible).
        if self.metatable().is_none() {
            return Ok(self.raw_len());
        }
        // Evaluate `#self` protected so a raising/returning `__len` is honored.
        let f = lua
            .load("local t = ...; return #t")
            .into_function()?;
        let n: i64 = f.call(self.clone())?;
        Ok(n.max(0) as usize)
    }

    /// Whether the table's sequence part is empty.
    pub fn is_empty(&self) -> bool {
        self.raw_len() == 0
    }

    /// Iterate over `(key, value)` pairs.
    ///
    /// Mirrors `mlua::Table::pairs`. Returns an iterator yielding `Result<(K,
    /// V)>` items. Uses `lua_next` under the hood.
    pub fn pairs<K: FromLua, V: FromLua>(&self) -> TablePairs<K, V> {
        TablePairs {
            table: self.clone(),
            next_key: Some(Value::Nil),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Collect all `(key, value)` pairs into a `Vec`. Convenience over
    /// [`Table::pairs`].
    pub fn pairs_vec<K: FromLua, V: FromLua>(&self) -> Result<Vec<(K, V)>> {
        self.pairs().collect()
    }

    /// Iterate over the sequence part `[1..]`, stopping at the first `nil`
    /// (raw access — ignores `__index`). Mirrors `mlua::Table::sequence_values`.
    pub fn sequence_values<V: FromLua>(&self) -> TableSequence<V> {
        TableSequence {
            table: self.clone(),
            index: 1,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Call `f` for each `(key, value)` pair (raw `lua_next` traversal).
    /// Stops early on the first `Err`. Mirrors `mlua::Table::for_each`.
    ///
    /// The key/value types are the only explicit type parameters (the closure
    /// type is inferred), matching mlua's `for_each::<K, V>(f)`.
    pub fn for_each<K: FromLua, V: FromLua>(
        &self,
        mut f: impl FnMut(K, V) -> Result<()>,
    ) -> Result<()> {
        for pair in self.pairs::<K, V>() {
            let (k, v) = pair?;
            f(k, v)?;
        }
        Ok(())
    }

    /// Call `f` for each value in the sequence part. Mirrors
    /// `mlua::Table::for_each_value`.
    pub fn for_each_value<V: FromLua>(
        &self,
        mut f: impl FnMut(V) -> Result<()>,
    ) -> Result<()> {
        for v in self.sequence_values::<V>() {
            f(v?)?;
        }
        Ok(())
    }

    // --- raw (metamethod-bypassing) access ---------------------------------

    /// Set `table[key] = value` without invoking `__newindex`.
    ///
    /// Mirrors `mlua::Table::raw_set`. Errors if the table is readonly.
    pub fn raw_set<K: IntoLua, V: IntoLua>(&self, key: K, value: V) -> Result<()> {
        let lua = self.lua();
        let state = lua.state();
        let k = key.into_lua(&lua)?;
        let v = value.into_lua(&lua)?;
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        unsafe {
            self.reference.push(); // table
            lua.push_value(&k)?; // key
            lua.push_value(&v)?; // value
            lua_rawset(state, -3);
            lua_pop(state, 1); // pop table
        }
        Ok(())
    }

    /// Get `table[key]` without invoking `__index`.
    ///
    /// Mirrors `mlua::Table::raw_get`.
    pub fn raw_get<V: FromLua>(&self, key: impl IntoLua) -> Result<V> {
        let lua = self.lua();
        let state = lua.state();
        let k = key.into_lua(&lua)?;
        let value = unsafe {
            self.reference.push(); // table
            lua.push_value(&k)?; // key
            lua_rawget(state, -2); // replaces key with value
            let v = lua.value_from_stack(-1)?;
            lua_pop(state, 2); // pop value + table
            v
        };
        V::from_lua(value, &lua)
    }

    /// Append `value` at position `#table + 1` using raw access.
    ///
    /// Mirrors `mlua::Table::raw_push`. Errors if readonly.
    pub fn raw_push<V: IntoLua>(&self, value: V) -> Result<()> {
        let n = self.raw_len();
        self.raw_set((n + 1) as i64, value)
    }

    /// Remove and return the last sequence element via raw access.
    ///
    /// Mirrors `mlua::Table::raw_pop`. Errors if readonly.
    pub fn raw_pop<V: FromLua>(&self) -> Result<V> {
        let lua = self.lua();
        let n = self.raw_len();
        if n == 0 {
            return V::from_lua(Value::Nil, &lua);
        }
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        let v: V = self.raw_get(n as i64)?;
        self.raw_set(n as i64, Value::Nil)?;
        Ok(v)
    }

    /// Insert `value` at 1-based `idx`, shifting later elements up (raw).
    ///
    /// Mirrors `mlua::Table::raw_insert`. Errors on bad index or readonly.
    pub fn raw_insert<V: IntoLua>(&self, idx: i64, value: V) -> Result<()> {
        let n = self.raw_len() as i64;
        if idx < 1 || idx > n + 1 {
            return Err(crate::error::Error::RuntimeError(format!(
                "bad argument #2 to 'insert' (position out of bounds): {idx}"
            )));
        }
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        // Shift [idx..=n] up by one, then place the new value.
        let mut i = n;
        while i >= idx {
            let moved: Value = self.raw_get(i)?;
            self.raw_set(i + 1, moved)?;
            i -= 1;
        }
        self.raw_set(idx, value.into_lua(&self.lua())?)
    }

    /// Remove and return the element at 1-based `idx`, shifting later
    /// elements down (raw). Mirrors `mlua::Table::raw_remove`.
    pub fn raw_remove(&self, idx: i64) -> Result<Value> {
        let n = self.raw_len() as i64;
        if n == 0 {
            return Ok(Value::Nil);
        }
        if idx < 1 || idx > n {
            return Err(crate::error::Error::RuntimeError(format!(
                "bad argument #1 to 'remove' (position out of bounds): {idx}"
            )));
        }
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        let removed: Value = self.raw_get(idx)?;
        let mut i = idx;
        while i < n {
            let moved: Value = self.raw_get(i + 1)?;
            self.raw_set(i, moved)?;
            i += 1;
        }
        self.raw_set(n, Value::Nil)?;
        Ok(removed)
    }

    /// Append `value` honoring `__len`/`__newindex` (uses `#self + 1`).
    /// Mirrors `mlua::Table::push`.
    pub fn push<V: IntoLua>(&self, value: V) -> Result<()> {
        let n = self.len()?;
        self.set((n + 1) as i64, value)
    }

    /// Pop the last element honoring `__len`/`__index`/`__newindex`.
    /// Mirrors `mlua::Table::pop`.
    pub fn pop<V: FromLua>(&self) -> Result<V> {
        let lua = self.lua();
        let n = self.len()?;
        if n == 0 {
            return V::from_lua(Value::Nil, &lua);
        }
        let v: V = self.get(n as i64)?;
        self.set(n as i64, Value::Nil)?;
        Ok(v)
    }

    /// Remove all keys from the table (raw). Errors if readonly.
    /// Mirrors `mlua::Table::clear`.
    pub fn clear(&self) -> Result<()> {
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        // Collect every key (raw traversal), then nil them out.
        let lua = self.lua();
        let state = lua.state();
        let mut keys: Vec<Value> = Vec::new();
        unsafe {
            self.reference.push(); // table
            lua_pushnil(state); // first key
            while lua_next(state, -2) != 0 {
                // stack: table, key, value
                let k = lua.value_from_stack(-2)?;
                keys.push(k);
                lua_pop(state, 1); // pop value, keep key for next iteration
            }
            lua_pop(state, 1); // pop table
        }
        for k in keys {
            self.raw_set(k, Value::Nil)?;
        }
        Ok(())
    }

    // --- identity / equality / metatables ----------------------------------

    /// A raw pointer identifying this table (for identity comparison).
    /// Mirrors `mlua::Table::to_pointer`.
    pub fn to_pointer(&self) -> *const std::ffi::c_void {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let p = lua_topointer(state, -1);
            lua_pop(state, 1);
            p
        }
    }

    /// Compare for equality honoring an `__eq` metamethod.
    /// Mirrors `mlua::Table::equals`.
    pub fn equals(&self, other: &Table) -> Result<bool> {
        let lua = self.lua();
        let state = lua.state();
        unsafe {
            self.reference.push();
            other.reference.push();
            let eq = lua_equal(state, -2, -1);
            lua_pop(state, 2);
            Ok(eq != 0)
        }
    }

    /// The table's metatable, if any. Mirrors `mlua::Table::metatable`.
    pub fn metatable(&self) -> Option<Table> {
        let lua = self.lua();
        let state = lua.state();
        unsafe {
            self.reference.push();
            let has = lua_getmetatable(state, -1);
            if has == 0 {
                lua_pop(state, 1); // pop table
                return None;
            }
            // stack: table, metatable
            let mt = Table::from_ref(lua.pop_ref());
            lua_pop(state, 1); // pop table
            Some(mt)
        }
    }

    /// Set (or clear, with `None`) the table's metatable.
    /// Mirrors `mlua::Table::set_metatable`. Errors if the table is readonly.
    pub fn set_metatable(&self, metatable: Option<Table>) -> Result<()> {
        if self.is_readonly() {
            return Err(crate::error::Error::RuntimeError(
                "attempt to modify a readonly table".to_string(),
            ));
        }
        let lua = self.lua();
        let state = lua.state();
        unsafe {
            self.reference.push(); // table
            match metatable {
                Some(mt) => mt.push_to_stack(),
                None => lua_pushnil(state),
            }
            lua_setmetatable(state, -2);
            lua_pop(state, 1); // pop table
        }
        Ok(())
    }

    // --- readonly (Luau extension) -----------------------------------------

    /// Whether the table is marked readonly (Luau). Mirrors
    /// `mlua::Table::is_readonly`.
    pub fn is_readonly(&self) -> bool {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let ro = lua_getreadonly(state, -1);
            lua_pop(state, 1);
            ro != 0
        }
    }

    /// Mark the table readonly or writable (Luau). Mirrors
    /// `mlua::Table::set_readonly`.
    pub fn set_readonly(&self, enabled: bool) {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            lua_setreadonly(state, -1, enabled as c_int);
            lua_pop(state, 1);
        }
    }
}

/// Iterator over a table's sequence part (see [`Table::sequence_values`]).
pub struct TableSequence<V> {
    table: Table,
    index: i64,
    _phantom: std::marker::PhantomData<V>,
}

impl<V: FromLua> Iterator for TableSequence<V> {
    type Item = Result<V>;

    fn next(&mut self) -> Option<Self::Item> {
        let lua = self.table.lua();
        // Raw get of the next sequence slot; stop at the first nil.
        let value: Value = match self.table.raw_get(self.index) {
            Ok(v) => v,
            Err(e) => return Some(Err(e)),
        };
        if value.is_nil() {
            return None;
        }
        self.index += 1;
        Some(V::from_lua(value, &lua))
    }
}

impl std::fmt::Debug for Table {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Table(len={})", self.raw_len())
    }
}

impl PartialEq for Table {
    fn eq(&self, other: &Self) -> bool {
        // Reference (pointer) identity, matching mlua's `==` on handles: two
        // handles are equal iff they point at the *same* Lua table object.
        self.to_pointer() == other.to_pointer()
    }
}

/// Compare a [`Table`]'s sequence part to a Rust slice of values.
///
/// Mirrors mlua's `impl PartialEq<[T]> for Table`: equal when the table's
/// `1..=len` sequence (read raw) matches `other` element-wise and has the
/// same length.
impl<T> PartialEq<[T]> for Table
where
    T: FromLua + PartialEq + Clone,
{
    fn eq(&self, other: &[T]) -> bool {
        // Compare the *sequence* part (stopping at the first nil border),
        // matching mlua's `sequence_values`-based slice comparison. This is the
        // robust border semantics for tables with nil holes (e.g.
        // `{1, 2, nil, 4, 5}` compares equal to `[1, 2]`).
        let mut iter = self.sequence_values::<T>();
        for expected in other.iter() {
            match iter.next() {
                Some(Ok(got)) if &got == expected => {}
                _ => return false,
            }
        }
        // The sequence must be exactly the slice length (no extra elements).
        iter.next().is_none()
    }
}

impl<T, const N: usize> PartialEq<[T; N]> for Table
where
    T: FromLua + PartialEq + Clone,
{
    fn eq(&self, other: &[T; N]) -> bool {
        self == other.as_slice()
    }
}

impl<T> PartialEq<&[T]> for Table
where
    T: FromLua + PartialEq + Clone,
{
    fn eq(&self, other: &&[T]) -> bool {
        self == *other
    }
}

/// Iterator over a table's key/value pairs (see [`Table::pairs`]).
pub struct TablePairs<K, V> {
    table: Table,
    next_key: Option<Value>,
    _phantom: std::marker::PhantomData<(K, V)>,
}

impl<K: FromLua, V: FromLua> Iterator for TablePairs<K, V> {
    type Item = Result<(K, V)>;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.next_key.take()?;
        let lua = self.table.lua();
        let state = lua.state();
        unsafe {
            self.table.reference.push(); // [.. table]
            if lua.push_value(&key).is_err() {
                lua_pop(state, 1);
                return None;
            }
            // stack: [table, key]
            let has = lua_next(state, -2);
            if has == 0 {
                // lua_next popped the key; pop the table.
                lua_pop(state, 1);
                self.next_key = None;
                return None;
            }
            // stack: [table, next_key, value]
            let k_val = match lua.value_from_stack(-2) {
                Ok(v) => v,
                Err(e) => {
                    lua_pop(state, 3);
                    return Some(Err(e));
                }
            };
            let v_val = match lua.value_from_stack(-1) {
                Ok(v) => v,
                Err(e) => {
                    lua_pop(state, 3);
                    return Some(Err(e));
                }
            };
            // Remember the key for the next iteration, then clean the stack.
            self.next_key = Some(k_val.clone());
            lua_pop(state, 3); // value, next_key, table

            let k = match K::from_lua(k_val, &lua) {
                Ok(k) => k,
                Err(e) => return Some(Err(e)),
            };
            let v = match V::from_lua(v_val, &lua) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            Some(Ok((k, v)))
        }
    }
}

/// Create a fresh empty table on `lua` and return a handle.
pub(crate) fn create_table(lua: &Lua) -> Table {
    let state = lua.state();
    unsafe {
        lua_createtable(state, 0, 0);
        Table::from_ref(lua.pop_ref())
    }
}

// ---------------------------------------------------------------------------
// Protected indexing
//
// `lua_gettable`/`lua_settable` may invoke `__index`/`__newindex` metamethods
// that *raise* (longjmp). Calling them unprotected across the Rust/VM boundary
// would unwind past Rust frames. We therefore run them inside `lua_pcall` via a
// tiny C trampoline, so a raising metamethod (or a readonly-table write) is
// reported as an ordinary non-zero status with the error object on the stack.
// ---------------------------------------------------------------------------

/// C trampoline: stack is `[table, key]`; performs `lua_gettable` and leaves
/// the result on top.
unsafe fn c_gettable(state: *mut lua_State) -> c_int {
    unsafe {
        lua_gettable(state, 1);
        1
    }
}

/// C trampoline: stack is `[table, key, value]`; performs `lua_settable`.
unsafe fn c_settable(state: *mut lua_State) -> c_int {
    unsafe {
        lua_settable(state, 1);
        0
    }
}

/// Run `lua_gettable` protected. Expects `[table, key]` on top; on success
/// leaves `[result]` where the two inputs were; on failure leaves the error
/// object on top and returns the non-zero status.
unsafe fn protected_gettable(state: *mut lua_State) -> c_int {
    unsafe {
        // Insert the C function below the two arguments: [t, k] -> [f, t, k].
        lua_pushcclosurek(
            state,
            Some(c_gettable),
            c"luaur-rt-gettable".as_ptr(),
            0,
            None,
        );
        lua_insert(state, -3);
        lua_pcall(state, 2, 1, 0)
    }
}

/// Run `lua_settable` protected. Expects `[table, key, value]` on top; pops
/// them on success; on failure leaves the error object and returns the status.
unsafe fn protected_settable(state: *mut lua_State) -> c_int {
    unsafe {
        // [t, k, v] -> [f, t, k, v].
        lua_pushcclosurek(
            state,
            Some(c_settable),
            c"luaur-rt-settable".as_ptr(),
            0,
            None,
        );
        lua_insert(state, -4);
        lua_pcall(state, 3, 0, 0)
    }
}
