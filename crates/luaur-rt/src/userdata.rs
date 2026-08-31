//! [`UserData`] / [`UserDataMethods`] / [`UserDataFields`] and the
//! [`AnyUserData`] handle. Mirrors `mlua::UserData` / `mlua::UserDataMethods` /
//! `mlua::UserDataFields` / `mlua::AnyUserData`.
//!
//! ## Implementation
//!
//! A `T: UserData` value is boxed into a Lua userdata as a typed wrapper
//! [`UserDataCell<T>`] = `{ type_id, RefCell<Option<T>> }` (via
//! [`lua_newuserdatadtor`], whose destructor drops the cell). The leading
//! `TypeId` makes Rust-side typed read-back **sound**: every accessor
//! ([`AnyUserData::borrow`], [`borrow_mut`](AnyUserData::borrow_mut),
//! [`take`](AnyUserData::take), [`is`](AnyUserData::is)) reads the stored
//! `TypeId` from the userdata pointer and compares it with `TypeId::of::<T>()`
//! before downcasting — a mismatch is an [`Error::UserDataTypeMismatch`].
//! `take` replaces the `Option<T>` with `None`; subsequent access reports
//! [`Error::UserDataDestructed`].
//!
//! Each registered method/field is compiled into a Rust closure wired into a
//! per-instance metatable:
//!   - ordinary methods go into a method table,
//!   - field getters/setters are dispatched by an `__index`/`__newindex`
//!     function (only when fields are registered),
//!   - meta-methods (e.g. `__add`) go directly on the metatable.

use std::any::TypeId;
use std::cell::RefCell;
use std::marker::PhantomData;

use crate::callback::{create_callback_function, BoxedCallback};
use crate::error::{Error, Result};
use crate::function::Function;
use crate::state::{Lua, LuaRef};
use crate::sync::{MaybeSend, MaybeSync, NotSync, XRc, NOT_SYNC};
use crate::sys::*;
use crate::table::Table;
use crate::traits::{FromLua, FromLuaMulti, IntoLua, IntoLuaMulti};
use crate::value::Value;

/// A Rust type that can be exposed to Lua as userdata.
///
/// Mirrors `mlua::UserData`. Implement [`UserData::add_methods`] and/or
/// [`UserData::add_fields`] to register the surface visible from Lua.
pub trait UserData: Sized {
    /// Register fields (getters/setters). Default: none.
    fn add_fields<F: UserDataFields<Self>>(_fields: &mut F) {}

    /// Register methods and meta-methods. Default: none.
    fn add_methods<M: UserDataMethods<Self>>(_methods: &mut M) {}
}

/// Registrar passed to [`UserData::add_methods`].
///
/// Mirrors `mlua::UserDataMethods`.
pub trait UserDataMethods<T> {
    /// Register a method callable as `obj:name(...)`; receives `&T`.
    fn add_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a method callable as `obj:name(...)`; receives `&mut T`.
    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a plain function in the userdata namespace (no `self`).
    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a meta-method (e.g. `MetaMethod::Add`, `"__tostring"`);
    /// receives `&T`.
    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a meta-method receiving `&mut T`.
    fn add_meta_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;
}

/// Registrar passed to [`UserData::add_fields`].
///
/// Mirrors `mlua::UserDataFields`. Field getters/setters are dispatched by the
/// userdata's `__index`/`__newindex`.
pub trait UserDataFields<T> {
    /// Register a constant field value (read-only).
    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + Clone + MaybeSend + 'static;

    /// Register a field whose getter receives `&T`.
    fn add_field_method_get<M, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T) -> Result<R> + MaybeSend + 'static,
        R: IntoLua;

    /// Register a field whose setter receives `&mut T` and the assigned value.
    fn add_field_method_set<M, A>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua;

    /// Register a field whose getter receives the [`AnyUserData`] handle.
    fn add_field_function_get<F, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData) -> Result<R> + MaybeSend + 'static,
        R: IntoLua;

    /// Register a field whose setter receives the [`AnyUserData`] handle and
    /// the assigned value.
    fn add_field_function_set<F, A>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua;
}

/// A handle to an arbitrary Lua userdata value.
///
/// Mirrors `mlua::AnyUserData`. Supports construction, use-from-Lua, and typed
/// Rust-side borrowing ([`borrow`](AnyUserData::borrow) /
/// [`borrow_mut`](AnyUserData::borrow_mut) / [`take`](AnyUserData::take) /
/// [`is`](AnyUserData::is)).
#[derive(Clone)]
pub struct AnyUserData {
    pub(crate) reference: XRc<LuaRef>,
    pub(crate) _not_sync: NotSync,
}

impl AnyUserData {
    pub(crate) fn from_ref(reference: LuaRef) -> AnyUserData {
        AnyUserData {
            reference: XRc::new(reference),
            _not_sync: NOT_SYNC,
        }
    }

    pub(crate) unsafe fn push_to_stack(&self) {
        self.reference.push();
    }

    /// The owning [`Lua`].
    pub fn lua(&self) -> Lua {
        self.reference.lua()
    }

    /// A raw pointer identifying this userdata. Mirrors
    /// `mlua::AnyUserData::to_pointer`.
    pub fn to_pointer(&self) -> *const c_void {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let p = lua_topointer(state, -1);
            lua_pop(state, 1);
            p
        }
    }

    /// Compare for equality honoring an `__eq` metamethod.
    /// Mirrors `mlua::AnyUserData::equals`.
    pub fn equals(&self, other: &AnyUserData) -> Result<bool> {
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

    /// Recover a `&UserDataCell<T>` from the userdata storage, checking the
    /// embedded `TypeId`. Returns `UserDataTypeMismatch` if the concrete type
    /// differs.
    fn cell<T: 'static>(&self) -> Result<&UserDataCell<T>> {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let ptr = lua_touserdata(state, -1);
            lua_pop(state, 1);
            if ptr.is_null() {
                return Err(Error::UserDataTypeMismatch);
            }
            // The wrapper stores the TypeId first; check it before downcasting.
            let header = &*(ptr as *const UserDataHeader);
            if header.type_id != TypeId::of::<T>() {
                return Err(Error::UserDataTypeMismatch);
            }
            Ok(&*(ptr as *const UserDataCell<T>))
        }
    }

    /// Whether the stored value is of concrete type `T`. Mirrors
    /// `mlua::AnyUserData::is`. Returns `false` after the value has been taken.
    pub fn is<T: 'static>(&self) -> bool {
        match self.cell::<T>() {
            Ok(cell) => cell.cell.borrow().is_some(),
            Err(_) => false,
        }
    }

    /// The [`TypeId`] of the stored value, if it is a luaur-rt userdata.
    /// Mirrors `mlua::AnyUserData::type_id` (here it returns the concrete
    /// `TypeId` whenever the userdata carries a luaur-rt wrapper header).
    pub fn type_id(&self) -> Option<TypeId> {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let ptr = lua_touserdata(state, -1);
            lua_pop(state, 1);
            if ptr.is_null() {
                return None;
            }
            // Only luaur-rt userdata carry a header; raw VM userdata do not, but
            // every userdata this crate creates does.
            let header = &*(ptr as *const UserDataHeader);
            Some(header.type_id)
        }
    }

    /// Immutably borrow the stored value as `T`. Mirrors
    /// `mlua::AnyUserData::borrow`. Errors with [`Error::UserDataTypeMismatch`]
    /// on a type mismatch, [`Error::UserDataDestructed`] if it was taken, or
    /// [`Error::UserDataBorrowError`] if already mutably borrowed.
    pub fn borrow<T: 'static>(&self) -> Result<UserDataRef<'_, T>> {
        let cell = self.cell::<T>()?;
        let guard = cell
            .cell
            .try_borrow()
            .map_err(|_| Error::UserDataBorrowError)?;
        if guard.is_none() {
            return Err(Error::UserDataDestructed);
        }
        Ok(UserDataRef {
            guard,
            _marker: PhantomData,
        })
    }

    /// Mutably borrow the stored value as `T`. Mirrors
    /// `mlua::AnyUserData::borrow_mut`.
    pub fn borrow_mut<T: 'static>(&self) -> Result<UserDataRefMut<'_, T>> {
        let cell = self.cell::<T>()?;
        let guard = cell
            .cell
            .try_borrow_mut()
            .map_err(|_| Error::UserDataBorrowMutError)?;
        if guard.is_none() {
            return Err(Error::UserDataDestructed);
        }
        Ok(UserDataRefMut {
            guard,
            _marker: PhantomData,
        })
    }

    /// Take the stored value out of the userdata, leaving it destructed.
    /// Mirrors `mlua::AnyUserData::take`. Errors with
    /// [`Error::UserDataBorrowMutError`] if currently borrowed, or
    /// [`Error::UserDataDestructed`] if already taken.
    pub fn take<T: 'static>(&self) -> Result<T> {
        let cell = self.cell::<T>()?;
        let mut guard = cell
            .cell
            .try_borrow_mut()
            .map_err(|_| Error::UserDataBorrowMutError)?;
        guard.take().ok_or(Error::UserDataDestructed)
    }
}

/// A RAII guard for an immutable userdata borrow ([`AnyUserData::borrow`]).
/// Mirrors `mlua::UserDataRef`.
pub struct UserDataRef<'a, T> {
    guard: std::cell::Ref<'a, Option<T>>,
    _marker: PhantomData<T>,
}

impl<T> std::ops::Deref for UserDataRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // Invariant: `borrow` returns only when the option is `Some`.
        self.guard.as_ref().expect("userdata destructed")
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for UserDataRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for UserDataRef<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

/// A RAII guard for a mutable userdata borrow ([`AnyUserData::borrow_mut`]).
/// Mirrors `mlua::UserDataRefMut`.
pub struct UserDataRefMut<'a, T> {
    guard: std::cell::RefMut<'a, Option<T>>,
    _marker: PhantomData<T>,
}

impl<T> std::ops::Deref for UserDataRefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.guard.as_ref().expect("userdata destructed")
    }
}

impl<T> std::ops::DerefMut for UserDataRefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.guard.as_mut().expect("userdata destructed")
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for UserDataRefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: std::fmt::Display> std::fmt::Display for UserDataRefMut<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&**self, f)
    }
}

impl std::fmt::Debug for AnyUserData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserData")
    }
}

impl PartialEq for AnyUserData {
    fn eq(&self, other: &Self) -> bool {
        // Pointer identity (matches mlua): same underlying userdata object.
        self.to_pointer() == other.to_pointer()
    }
}

// Any `T: UserData` value converts into Lua by wrapping it in a fresh userdata.
// Mirrors mlua's `impl<T: UserData + MaybeSend + MaybeSync + 'static> IntoLua`.
// This is what lets `create_registry_value(MyUserdata(..))`,
// `table.set("k", MyUserdata(..))`, etc. accept a userdata value directly. It
// coexists with the concrete `IntoLua` impls below because none of those
// (local) types implement `UserData`.
impl<T: UserData + crate::sync::MaybeSend + crate::sync::MaybeSync + 'static> IntoLua for T {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(lua.create_userdata(self)?))
    }
}

impl IntoLua for AnyUserData {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(self))
    }
}

impl IntoLua for &AnyUserData {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(self.clone()))
    }
}

impl FromLua for AnyUserData {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "AnyUserData".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed userdata storage
// ---------------------------------------------------------------------------

/// The fixed leading layout of every luaur-rt userdata wrapper. Reading the
/// `TypeId` through this header (a prefix of [`UserDataCell<T>`]) lets us check
/// the concrete type before downcasting. The two share `#[repr(C)]` so the
/// `type_id` field is at the same offset for any `T`.
#[repr(C)]
struct UserDataHeader {
    type_id: TypeId,
}

/// The typed userdata storage: a `TypeId` followed by the `RefCell<Option<T>>`.
#[repr(C)]
struct UserDataCell<T> {
    type_id: TypeId,
    cell: RefCell<Option<T>>,
}

/// Recover `&UserDataCell<T>` from a `self` userdata [`Value`] (Lua argument 1).
fn recover_cell<'a, T: 'static>(lua: &Lua, value: &Value) -> Result<&'a UserDataCell<T>> {
    match value {
        Value::UserData(ud) => {
            let state = lua.state();
            unsafe {
                ud.reference.push();
                let ptr = lua_touserdata(state, -1);
                lua_pop(state, 1);
                if ptr.is_null() {
                    return Err(Error::UserDataTypeMismatch);
                }
                let header = &*(ptr as *const UserDataHeader);
                if header.type_id != TypeId::of::<T>() {
                    return Err(Error::UserDataTypeMismatch);
                }
                Ok(&*(ptr as *const UserDataCell<T>))
            }
        }
        _ => Err(Error::UserDataTypeMismatch),
    }
}

// ---------------------------------------------------------------------------
// Method collection
// ---------------------------------------------------------------------------

/// A registered method or meta-method, paired with its name and whether it is a
/// meta-method.
struct Registered {
    name: String,
    is_meta: bool,
    callback: BoxedCallback,
}

/// A registered field getter or setter.
struct FieldEntry {
    name: String,
    /// `true` if a getter, `false` if a setter.
    is_get: bool,
    callback: BoxedCallback,
}

/// Concrete [`UserDataMethods`] / [`UserDataFields`] implementation that
/// collects the type-erased callbacks; the metatable is built from these.
struct Collector<T> {
    methods: Vec<Registered>,
    fields: Vec<FieldEntry>,
    _phantom: PhantomData<T>,
}

impl<T> Collector<T> {
    fn new() -> Self {
        Collector {
            methods: Vec::new(),
            fields: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<T: 'static> UserDataMethods<T> for Collector<T> {
    fn add_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, args| {
            let a = A::from_lua_multi(args, lua)?;
            let r = function(lua, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: true,
            callback,
        });
    }

    fn add_meta_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: true,
            callback,
        });
    }
}

impl<T: 'static> UserDataFields<T> for Collector<T> {
    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + Clone + MaybeSend + 'static,
    {
        let callback: BoxedCallback = Box::new(move |lua, _args| {
            let v = value.clone().into_lua(lua)?;
            v.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_method_get<M, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T) -> Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data)?;
            r.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_method_set<M, A>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let val = A::from_lua(args.pop_front().unwrap_or(Value::Nil), lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            method(lua, data, val)?;
            ().into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: false,
            callback,
        });
    }

    fn add_field_function_get<F, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData) -> Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let ud = AnyUserData::from_lua(this, lua)?;
            let r = function(lua, ud)?;
            r.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_function_set<F, A>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let ud = AnyUserData::from_lua(this, lua)?;
            let val = A::from_lua(args.pop_front().unwrap_or(Value::Nil), lua)?;
            function(lua, ud, val)?;
            ().into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: false,
            callback,
        });
    }
}

/// Destructor for the [`UserDataCell<T>`] stored inside the userdata.
unsafe extern "C" fn userdata_dtor<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut UserDataCell<T>) };
    }
}

// ---------------------------------------------------------------------------
// Scoped (non-'static) userdata — used by `Lua::scope`
// ---------------------------------------------------------------------------
//
// Ordinary userdata stores a `TypeId` so values can be soundly read back out by
// concrete type (`borrow`/`take`/`is`). That requires `T: 'static`.
//
// A scope can create userdata wrapping a **non-`'static`** `T` (e.g. one that
// borrows from the enclosing stack frame). For these there is no `TypeId`, so
// instead each scoped userdata is tagged with a process-unique `u64` **marker**.
// Every method/field/meta closure for that userdata captures the *same* marker,
// so on dispatch it can confirm the `self` it received is exactly the userdata
// it belongs to (recovering `&ScopedCell<T>` is sound only after the marker
// matches — markers are never reused, so no other userdata can collide).
//
// Soundness over the scope lifetime: while the scope is active the wrapped `T`
// is `Some` and methods may form a transient `&T`/`&mut T`. On scope exit the
// scope's destructor `take()`s the value to `None` (dropping the borrowed `T`,
// ending its borrows) but leaves the cell memory valid; any later dispatch finds
// `None` and returns `Error::UserDataDestructed`. The cell itself is freed only
// when the GC collects the userdata, never while a `&ScopedCell<T>` could exist.

use std::sync::atomic::{AtomicU64, Ordering};

/// Source of process-unique scoped-userdata markers.
static SCOPED_MARKER: AtomicU64 = AtomicU64::new(1);

fn next_scoped_marker() -> u64 {
    SCOPED_MARKER.fetch_add(1, Ordering::Relaxed)
}

/// The fixed leading layout of a scoped userdata wrapper: a unique marker used
/// to recognise the instance (in place of a `TypeId`).
#[repr(C)]
struct ScopedHeader {
    marker: u64,
}

/// Scoped userdata storage: a unique marker followed by the data cell. Shares
/// `#[repr(C)]` with [`ScopedHeader`] so the marker is at offset 0 for any `T`.
#[repr(C)]
struct ScopedCell<T> {
    marker: u64,
    cell: RefCell<Option<T>>,
}

/// Destructor for the [`ScopedCell<T>`] stored inside a scoped userdata.
unsafe extern "C" fn scoped_userdata_dtor<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut ScopedCell<T>) };
    }
}

/// Recover `&ScopedCell<T>` from a `self` userdata [`Value`] (Lua argument 1),
/// verifying the per-instance `marker`. A mismatch is an
/// [`Error::UserDataTypeMismatch`].
///
/// # Safety
/// The caller guarantees that any userdata carrying `marker` was created as a
/// `ScopedCell<T>` for this exact `T` (which holds because each marker is handed
/// out to exactly one `create_scoped_userdata::<T>` call).
unsafe fn recover_scoped_cell<'a, T>(
    lua: &Lua,
    value: &Value,
    marker: u64,
) -> Result<&'a ScopedCell<T>> {
    match value {
        Value::UserData(ud) => {
            let state = lua.state();
            unsafe {
                ud.reference.push();
                let ptr = lua_touserdata(state, -1);
                lua_pop(state, 1);
                if ptr.is_null() {
                    return Err(Error::UserDataTypeMismatch);
                }
                let header = &*(ptr as *const ScopedHeader);
                if header.marker != marker {
                    return Err(Error::UserDataTypeMismatch);
                }
                Ok(&*(ptr as *const ScopedCell<T>))
            }
        }
        _ => Err(Error::UserDataTypeMismatch),
    }
}

/// A concrete [`UserDataMethods`] / [`UserDataFields`] implementation for a
/// scoped (non-`'static`) userdata instance: identical surface to [`Collector`],
/// but it recovers the data cell by the per-instance `marker` rather than a
/// `TypeId`, so it works for non-`'static` `T`.
struct ScopedCollector<T> {
    marker: u64,
    methods: Vec<Registered>,
    fields: Vec<FieldEntry>,
    _phantom: PhantomData<T>,
}

impl<T> ScopedCollector<T> {
    fn new(marker: u64) -> Self {
        ScopedCollector {
            marker,
            methods: Vec::new(),
            fields: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

impl<T> UserDataMethods<T> for ScopedCollector<T> {
    fn add_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let a = A::from_lua_multi(args, lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, args| {
            let a = A::from_lua_multi(args, lua)?;
            let r = function(lua, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: true,
            callback,
        });
    }

    fn add_meta_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let a = A::from_lua_multi(args, lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data, a)?;
            r.into_lua_multi(lua)
        });
        self.methods.push(Registered {
            name: name.into(),
            is_meta: true,
            callback,
        });
    }
}

impl<T> UserDataFields<T> for ScopedCollector<T> {
    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + Clone + MaybeSend + 'static,
    {
        let callback: BoxedCallback = Box::new(move |lua, _args| {
            let v = value.clone().into_lua(lua)?;
            v.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_method_get<M, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T) -> Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let borrowed = cell
                .cell
                .try_borrow()
                .map_err(|_| Error::UserDataBorrowError)?;
            let data = borrowed.as_ref().ok_or(Error::UserDataDestructed)?;
            let r = method(lua, data)?;
            r.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_method_set<M, A>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let marker = self.marker;
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = unsafe { recover_scoped_cell::<T>(lua, &this, marker)? };
            let val = A::from_lua(args.pop_front().unwrap_or(Value::Nil), lua)?;
            let mut borrowed = cell
                .cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let data = borrowed.as_mut().ok_or(Error::UserDataDestructed)?;
            method(lua, data, val)?;
            ().into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: false,
            callback,
        });
    }

    fn add_field_function_get<F, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData) -> Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let ud = AnyUserData::from_lua(this, lua)?;
            let r = function(lua, ud)?;
            r.into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: true,
            callback,
        });
    }

    fn add_field_function_set<F, A>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData, A) -> Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let ud = AnyUserData::from_lua(this, lua)?;
            let val = A::from_lua(args.pop_front().unwrap_or(Value::Nil), lua)?;
            function(lua, ud, val)?;
            ().into_lua_multi(lua)
        });
        self.fields.push(FieldEntry {
            name: name.into(),
            is_get: false,
            callback,
        });
    }
}

/// Install the userdata metatable's `__index`.
///
/// Mirrors mlua's generated `__index` (`mlua::userdata::util`): a lookup falls
/// through **field getters → the method table → a user-registered `__index`
/// meta-method**. A custom `__index` set by [`UserDataMethods::add_meta_method`]
/// therefore stays reachable instead of being overwritten by the method table
/// or the field dispatcher.
///
/// The cheap case — no fields and no custom `__index` — keeps `__index` as the
/// method table itself, so ordinary method lookup stays a plain table access.
///
/// DEVIATION from mlua: an unresolved key yields `nil` rather than raising
/// "attempt to get an unknown field", matching luaur-rt's existing behavior.
fn install_index_dispatcher(
    lua: &Lua,
    metatable: &Table,
    method_table: Table,
    getters: Table,
    has_fields: bool,
) -> Result<()> {
    // A custom `__index` was written onto the metatable by the meta-method loop
    // above; capture it here so the dispatcher can fall back to it. Only
    // functions can land there (`add_meta_method`/`add_meta_method_mut` are the
    // only ways in), so a non-function `__index` is not representable.
    let user_index: Option<Function> = metatable.raw_get("__index")?;

    if !has_fields && user_index.is_none() {
        metatable.set("__index", method_table)?;
        return Ok(());
    }

    let index_fn = lua.create_function(move |_, (ud, key): (Value, Value)| {
        if has_fields {
            let getter: Value = getters.get(key.clone())?;
            if let Value::Function(f) = getter {
                return f.call::<Value>(ud);
            }
        }
        let method: Value = method_table.get(key.clone())?;
        if method != Value::Nil {
            return Ok(method);
        }
        match &user_index {
            Some(f) => f.call::<Value>((ud, key)),
            None => Ok(Value::Nil),
        }
    })?;
    metatable.set("__index", index_fn)?;
    Ok(())
}

/// Build a scoped (non-`'static`) userdata wrapping `data`, with a metatable
/// assembled from `T::add_fields` + `T::add_methods`. Returns the
/// [`AnyUserData`] handle plus a closure that, when called, neutralises the
/// userdata (drops `data`, leaving later access to error with
/// [`Error::UserDataDestructed`]). The neutraliser is what `Lua::scope`
/// registers as a destructor.
///
/// # Safety
/// The returned [`AnyUserData`] must not be used to read `data` back out by
/// type (there is no `TypeId`); only metatable-driven method/field/meta dispatch
/// is supported. The scope must invoke the returned neutraliser before `data`'s
/// borrowed lifetime ends.
pub(crate) fn create_scoped_userdata<T: UserData>(
    lua: &Lua,
    data: T,
) -> Result<(AnyUserData, Box<dyn FnOnce()>)> {
    let state = lua.state();
    let marker = next_scoped_marker();

    // 1. Collect fields, methods, and meta-methods (marker-keyed recovery).
    let mut collector = ScopedCollector::<T>::new(marker);
    T::add_fields(&mut collector);
    T::add_methods(&mut collector);

    // 2. Build the method table + meta-methods + field getter/setter tables.
    let method_table = lua.create_table();
    let metatable = lua.create_table();
    for item in collector.methods {
        let func = create_callback_function(lua, item.callback)?;
        if item.is_meta {
            metatable.set(item.name, func)?;
        } else {
            method_table.set(item.name, func)?;
        }
    }

    let has_fields = !collector.fields.is_empty();
    let getters = lua.create_table();
    let setters = lua.create_table();
    for field in collector.fields {
        let func = create_callback_function(lua, field.callback)?;
        if field.is_get {
            getters.set(field.name, func)?;
        } else {
            setters.set(field.name, func)?;
        }
    }

    if has_fields {
        let setters_c = setters.clone();
        let newindex_fn =
            lua.create_function(move |_, (ud, key, val): (Value, Value, Value)| {
                let setter: Value = setters_c.get(key.clone())?;
                if let Value::Function(f) = setter {
                    f.call::<()>((ud, val))?;
                    return Ok(());
                }
                let name = key.to_string().unwrap_or_default();
                Err(Error::RuntimeError(format!(
                    "attempt to set unknown field '{name}' on userdata"
                )))
            })?;
        metatable.set("__newindex", newindex_fn)?;
    }
    install_index_dispatcher(lua, &metatable, method_table, getters, has_fields)?;

    // 3. Allocate the scoped userdata holding ScopedCell<T> and move `data` in.
    let ud = unsafe {
        let storage = lua_newuserdatadtor(
            state,
            core::mem::size_of::<ScopedCell<T>>(),
            Some(scoped_userdata_dtor::<T>),
        );
        if storage.is_null() {
            return Err(Error::runtime(
                "luaur-rt: failed to allocate scoped userdata",
            ));
        }
        core::ptr::write(
            storage as *mut ScopedCell<T>,
            ScopedCell {
                marker,
                cell: RefCell::new(Some(data)),
            },
        );
        metatable.push_to_stack();
        lua_setmetatable(state, -2);
        AnyUserData::from_ref(lua.pop_ref())
    };

    // 4. Build the neutraliser: on scope exit, take the data out of the cell,
    //    dropping the (possibly borrowing) `T` while the cell memory stays valid.
    let ud_for_dtor = ud.clone();
    let neutralise: Box<dyn FnOnce()> = Box::new(move || {
        let state = ud_for_dtor.reference.state();
        unsafe {
            ud_for_dtor.reference.push();
            let ptr = lua_touserdata(state, -1);
            lua_pop(state, 1);
            if ptr.is_null() {
                return;
            }
            let cell = &*(ptr as *const ScopedCell<T>);
            // Drop the data (ends borrows). If currently borrowed (a method is
            // somehow live), `try_borrow_mut` fails and we leave it — but scope
            // exit only happens after `f` returns, so no method is in flight.
            if let Ok(mut guard) = cell.cell.try_borrow_mut() {
                let _ = guard.take();
            }
        }
    });

    Ok((ud, neutralise))
}

/// Build a userdata value wrapping `data`, with a metatable assembled from the
/// type's [`UserData::add_fields`] + [`UserData::add_methods`].
pub(crate) fn create_userdata<T: UserData + MaybeSend + MaybeSync + 'static>(
    lua: &Lua,
    data: T,
) -> Result<AnyUserData> {
    let state = lua.state();

    // 1. Collect fields, methods, and meta-methods.
    let mut collector = Collector::<T>::new();
    T::add_fields(&mut collector);
    T::add_methods(&mut collector);

    // 2. Build the method table + meta-methods, and the field getter/setter
    //    tables (if any fields were registered).
    let method_table = lua.create_table();
    let metatable = lua.create_table();
    for item in collector.methods {
        let func = create_callback_function(lua, item.callback)?;
        if item.is_meta {
            metatable.set(item.name, func)?;
        } else {
            method_table.set(item.name, func)?;
        }
    }

    let has_fields = !collector.fields.is_empty();
    let getters = lua.create_table();
    let setters = lua.create_table();
    for field in collector.fields {
        let func = create_callback_function(lua, field.callback)?;
        if field.is_get {
            getters.set(field.name, func)?;
        } else {
            setters.set(field.name, func)?;
        }
    }

    if has_fields {
        // __newindex dispatcher: try a field setter, else raise.
        let setters_c = setters.clone();
        let newindex_fn =
            lua.create_function(move |_, (ud, key, val): (Value, Value, Value)| {
                let setter: Value = setters_c.get(key.clone())?;
                if let Value::Function(f) = setter {
                    f.call::<()>((ud, val))?;
                    return Ok(());
                }
                let name = key.to_string().unwrap_or_default();
                Err(Error::RuntimeError(format!(
                    "attempt to set unknown field '{name}' on userdata"
                )))
            })?;
        metatable.set("__newindex", newindex_fn)?;
    }
    // __index: field getters -> method table -> a custom `__index` meta-method.
    install_index_dispatcher(lua, &metatable, method_table, getters, has_fields)?;

    // 3. Allocate the userdata holding UserDataCell<T> and move `data` in.
    unsafe {
        let storage = lua_newuserdatadtor(
            state,
            core::mem::size_of::<UserDataCell<T>>(),
            Some(userdata_dtor::<T>),
        );
        if storage.is_null() {
            return Err(Error::runtime("luaur-rt: failed to allocate userdata"));
        }
        core::ptr::write(
            storage as *mut UserDataCell<T>,
            UserDataCell {
                type_id: TypeId::of::<T>(),
                cell: RefCell::new(Some(data)),
            },
        );

        // 4. Set the metatable on the userdata (which is on top of stack).
        metatable.push_to_stack();
        lua_setmetatable(state, -2);

        // 5. Take a ref to the userdata and return.
        Ok(AnyUserData::from_ref(lua.pop_ref()))
    }
}
