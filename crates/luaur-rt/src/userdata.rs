//! [`UserData`] / [`UserDataMethods`] and the [`AnyUserData`] handle.
//! Mirrors `mlua::UserData` / `mlua::UserDataMethods` / `mlua::AnyUserData`.
//!
//! ## Implementation
//!
//! A `T: UserData` value is boxed into a Lua userdata as `RefCell<T>` (via
//! [`lua_newuserdatadtor`], whose destructor drops the cell). Each registered
//! method is compiled into a Rust closure that:
//!   - takes the userdata as the first Lua argument (`self`),
//!   - recovers `&RefCell<T>` from the userdata pointer,
//!   - borrows (`add_method`) or mutably borrows (`add_method_mut`) it,
//!   - calls the user method with the remaining arguments.
//!
//! These closures are wired into a per-instance metatable: ordinary methods go
//! into a `__index` sub-table (so `obj:method()` resolves), `__meta` methods
//! (e.g. `__add`) go directly on the metatable.
//!
//! ### Deviations from mlua
//! `UserDataFields` (field getters/setters via `__index`/`__newindex`
//! functions) is **deferred** — see the crate docs. `AnyUserData::borrow` for
//! typed read-back from Rust is also deferred; the v1 surface focuses on
//! constructing userdata and using it *from Lua*.

use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::callback::{create_callback_function, BoxedCallback};
use crate::error::{Error, Result};
use crate::ffi::*;
use crate::state::{Lua, LuaRef};
use crate::traits::{FromLuaMulti, IntoLua, IntoLuaMulti};
use crate::value::Value;

/// A Rust type that can be exposed to Lua as userdata.
///
/// Mirrors `mlua::UserData`. Implement [`UserData::add_methods`] to register
/// methods and meta-methods.
pub trait UserData: Sized {
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
        M: Fn(&Lua, &T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a method callable as `obj:name(...)`; receives `&mut T`.
    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a plain function in the userdata namespace (no `self`).
    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;

    /// Register a meta-method (e.g. `"__add"`, `"__tostring"`); receives `&T`.
    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti;
}

/// A handle to an arbitrary Lua userdata value.
///
/// Mirrors `mlua::AnyUserData`. v1 exposes construction + use-from-Lua;
/// typed Rust-side borrowing is deferred.
#[derive(Clone)]
pub struct AnyUserData {
    pub(crate) reference: Rc<LuaRef>,
}

impl AnyUserData {
    pub(crate) fn from_ref(reference: LuaRef) -> AnyUserData {
        AnyUserData {
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

impl IntoLua for AnyUserData {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::UserData(self))
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

/// Concrete [`UserDataMethods`] implementation that simply collects the
/// type-erased callbacks; the metatable is built from this collection.
struct Collector<T> {
    items: Vec<Registered>,
    _phantom: PhantomData<T>,
}

impl<T> Collector<T> {
    fn new() -> Self {
        Collector {
            items: Vec::new(),
            _phantom: PhantomData,
        }
    }
}

/// Recover `&RefCell<T>` from the `self` userdata value (Lua argument 1).
///
/// Returns an error if the argument is not the expected userdata.
fn recover_cell<'a, T: 'static>(lua: &Lua, value: &Value) -> Result<&'a RefCell<T>> {
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
                Ok(&*(ptr as *const RefCell<T>))
            }
        }
        _ => Err(Error::UserDataTypeMismatch),
    }
}

impl<T: 'static> UserDataMethods<T> for Collector<T> {
    fn add_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell.borrow();
            let r = method(lua, &borrowed, a)?;
            r.into_lua_multi(lua)
        });
        self.items.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &mut T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let mut borrowed = cell
                .try_borrow_mut()
                .map_err(|_| Error::UserDataBorrowMutError)?;
            let r = method(lua, &mut borrowed, a)?;
            r.into_lua_multi(lua)
        });
        self.items.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, args| {
            let a = A::from_lua_multi(args, lua)?;
            let r = function(lua, a)?;
            r.into_lua_multi(lua)
        });
        self.items.push(Registered {
            name: name.into(),
            is_meta: false,
            callback,
        });
    }

    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> Result<R> + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let callback: BoxedCallback = Box::new(move |lua, mut args| {
            let this = args.pop_front().unwrap_or(Value::Nil);
            let cell = recover_cell::<T>(lua, &this)?;
            let a = A::from_lua_multi(args, lua)?;
            let borrowed = cell.borrow();
            let r = method(lua, &borrowed, a)?;
            r.into_lua_multi(lua)
        });
        self.items.push(Registered {
            name: name.into(),
            is_meta: true,
            callback,
        });
    }
}

/// Destructor for the `RefCell<T>` stored inside the userdata.
unsafe extern "C" fn userdata_dtor<T>(ptr: *mut c_void) {
    if !ptr.is_null() {
        unsafe { core::ptr::drop_in_place(ptr as *mut RefCell<T>) };
    }
}

/// Build a userdata value wrapping `data`, with a metatable assembled from the
/// type's [`UserData::add_methods`]. Implements [`Lua::create_userdata`].
pub(crate) fn create_userdata<T: UserData + 'static>(lua: &Lua, data: T) -> Result<AnyUserData> {
    let state = lua.state();

    // 1. Collect methods/meta-methods.
    let mut collector = Collector::<T>::new();
    T::add_methods(&mut collector);

    // 2. Build the `__index` table of ordinary methods and a list of
    //    meta-methods to set directly on the metatable.
    let index_table = lua.create_table();
    let metatable = lua.create_table();
    for item in collector.items {
        let func = create_callback_function(lua, item.callback)?;
        if item.is_meta {
            metatable.set(item.name, func)?;
        } else {
            index_table.set(item.name, func)?;
        }
    }
    // metatable.__index = index_table
    metatable.set("__index", index_table)?;

    // 3. Allocate the userdata holding RefCell<T> and move `data` in.
    unsafe {
        let storage = lua_newuserdatadtor(
            state,
            core::mem::size_of::<RefCell<T>>(),
            Some(userdata_dtor::<T>),
        );
        if storage.is_null() {
            return Err(Error::runtime("luaur-rt: failed to allocate userdata"));
        }
        core::ptr::write(storage as *mut RefCell<T>, RefCell::new(data));

        // 4. Set the metatable on the userdata (which is on top of stack).
        //    push metatable, then setmetatable(-2).
        metatable.push_to_stack();
        lua_setmetatable(state, -2);

        // 5. Take a ref to the userdata and return.
        Ok(AnyUserData::from_ref(lua.pop_ref()))
    }
}
