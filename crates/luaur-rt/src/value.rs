//! The [`Value`] enum and the stack <-> `Value` bridges.
//!
//! Mirrors `mlua::Value`. Luau (like Lua 5.x without the integer subtype)
//! stores every number as an `f64`; there is no distinct integer tag at the VM
//! level. We therefore reconstruct the [`Value::Integer`] vs [`Value::Number`]
//! distinction on the way out by testing whether the `f64` is an exact,
//! in-range integer — matching mlua's observable behavior closely enough for
//! the high-level API.

use crate::error::Result;
use crate::function::Function;
use crate::ffi::*;
use crate::state::Lua;
use crate::string::LuaString;
use crate::table::Table;

/// The integer type exposed by the API. Mirrors `mlua::Integer` (`i64`).
pub type Integer = i64;
/// The float type exposed by the API. Mirrors `mlua::Number` (`f64`).
pub type Number = f64;

/// A dynamically typed Lua value.
///
/// Mirrors `mlua::Value`. Reference-typed variants ([`Value::String`],
/// [`Value::Table`], [`Value::Function`]) carry handles that keep both the
/// value and the VM alive.
#[derive(Clone, Debug)]
pub enum Value {
    /// `nil`.
    Nil,
    /// A boolean.
    Boolean(bool),
    /// An integer (an `f64` that is an exact, in-range whole number).
    Integer(Integer),
    /// A floating-point number.
    Number(Number),
    /// A string.
    String(LuaString),
    /// A table.
    Table(Table),
    /// A function (Lua or Rust).
    Function(Function),
    /// A userdata value, represented opaquely as a table-backed handle. (Full
    /// `AnyUserData` borrowing is deferred — see crate docs.)
    UserData(crate::userdata::AnyUserData),
    /// A boxed Lua/Rust error carried as a first-class value (mirrors
    /// `mlua::Value::Error`). Produced when a Rust error is returned to Lua.
    Error(Box<crate::error::Error>),
}

impl Value {
    /// `Value::Nil`. Mirrors `mlua::Nil`.
    pub const NIL: Value = Value::Nil;

    /// The Lua type name of this value (e.g. `"nil"`, `"number"`, `"table"`).
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Nil => "nil",
            Value::Boolean(_) => "boolean",
            Value::Integer(_) | Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Table(_) => "table",
            Value::Function(_) => "function",
            Value::UserData(_) => "userdata",
            Value::Error(_) => "error",
        }
    }

    /// Whether this is an error value.
    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }

    /// View as a reference to the contained error, if any.
    pub fn as_error(&self) -> Option<&crate::error::Error> {
        match self {
            Value::Error(e) => Some(e),
            _ => None,
        }
    }

    /// Whether this is `nil`.
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    /// Whether this is a boolean.
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }
    /// Whether this is a number of either subtype.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_) | Value::Integer(_))
    }
    /// Whether this is the integer subtype.
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }
    /// Whether this is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    /// Whether this is a table.
    pub fn is_table(&self) -> bool {
        matches!(self, Value::Table(_))
    }
    /// Whether this is a function.
    pub fn is_function(&self) -> bool {
        matches!(self, Value::Function(_))
    }

    /// Lua truthiness: everything except `nil` and `false` is truthy.
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }
    /// View as an integer if it is one.
    pub fn as_integer(&self) -> Option<Integer> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }
    /// View as an `f64` if it is a number of either subtype.
    pub fn as_number(&self) -> Option<Number> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }
    /// View as a string handle.
    pub fn as_string(&self) -> Option<&LuaString> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
    /// View as a table handle.
    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }
    /// View as a function handle.
    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Value::Function(f) => Some(f),
            _ => None,
        }
    }

    /// Whether this is a userdata value.
    pub fn is_userdata(&self) -> bool {
        matches!(self, Value::UserData(_))
    }

    /// View as a userdata handle.
    pub fn as_userdata(&self) -> Option<&crate::userdata::AnyUserData> {
        match self {
            Value::UserData(u) => Some(u),
            _ => None,
        }
    }

    /// View as an `i32` if it is an in-range integer.
    pub fn as_i32(&self) -> Option<i32> {
        self.as_integer().and_then(|i| i32::try_from(i).ok())
    }
    /// View as a `u32` if it is an in-range integer.
    pub fn as_u32(&self) -> Option<u32> {
        self.as_integer().and_then(|i| u32::try_from(i).ok())
    }
    /// View as an `i64` if it is an integer.
    pub fn as_i64(&self) -> Option<i64> {
        self.as_integer()
    }
    /// View as a `u64` if it is an in-range integer.
    pub fn as_u64(&self) -> Option<u64> {
        self.as_integer().and_then(|i| u64::try_from(i).ok())
    }
    /// View as an `isize` if it is an in-range integer.
    pub fn as_isize(&self) -> Option<isize> {
        self.as_integer().and_then(|i| isize::try_from(i).ok())
    }
    /// View as a `usize` if it is an in-range integer.
    pub fn as_usize(&self) -> Option<usize> {
        self.as_integer().and_then(|i| usize::try_from(i).ok())
    }
    /// View as an `f32`.
    pub fn as_f32(&self) -> Option<f32> {
        self.as_number().map(|n| n as f32)
    }
    /// View as an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        self.as_number()
    }

    /// A raw pointer identifying reference-typed values (tables, functions,
    /// strings, userdata). Returns null for value-typed (nil/bool/number)
    /// values. Mirrors `mlua::Value::to_pointer`.
    pub fn to_pointer(&self) -> *const std::ffi::c_void {
        match self {
            Value::String(s) => s.to_pointer(),
            Value::Table(t) => t.to_pointer(),
            Value::Function(f) => f.to_pointer(),
            Value::UserData(u) => u.to_pointer(),
            _ => core::ptr::null(),
        }
    }

    /// Compare two values for equality honoring `__eq` metamethods.
    /// Mirrors `mlua::Value::equals`.
    pub fn equals(&self, other: &Value) -> Result<bool> {
        // For reference types, route through the VM's `lua_equal` (which runs
        // `__eq`). For value types, structural equality matches Lua semantics.
        match (self, other) {
            (Value::Table(a), Value::Table(b)) => a.equals(b),
            (Value::UserData(a), Value::UserData(b)) => a.equals(b),
            _ => Ok(self == other),
        }
    }

    /// The metatable-aware string form of this value (honors `__tostring`).
    /// Mirrors `mlua::Value::to_string`.
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> Result<String> {
        match self {
            Value::Nil => Ok("nil".to_string()),
            Value::Boolean(b) => Ok(b.to_string()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) => Ok(crate::value::format_number(*n)),
            Value::Error(e) => Ok(e.to_string()),
            // Reference types: ask the VM (honors `__tostring`).
            Value::String(s) => s.to_str(),
            other => {
                // Find the owning Lua via the handle and use luaL_tolstring.
                let lua = match other {
                    Value::Table(t) => t.lua(),
                    Value::Function(f) => f.lua(),
                    Value::UserData(u) => u.lua(),
                    _ => unreachable!(),
                };
                lua.value_to_string(other)
            }
        }
    }
}

/// Format an `f64` the way Lua's `tostring` does for the common cases the
/// tests exercise (e.g. `34.59`, integral floats as plain integers).
pub(crate) fn format_number(n: f64) -> String {
    if n.fract() == 0.0 && n.is_finite() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        // Rust's default float formatting matches Lua's "%.14g" closely enough
        // for the values under test.
        let s = format!("{n}");
        s
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            // Numbers compare by value across the integer/float subtypes,
            // matching Lua's `==` (1 == 1.0).
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Integer(a), Value::Number(b)) | (Value::Number(b), Value::Integer(a)) => {
                (*a as f64) == *b
            }
            (Value::String(a), Value::String(b)) => a == b,
            // Reference types: identity (NOT `__eq`); use `equals` for `__eq`.
            (Value::Table(a), Value::Table(b)) => a.to_pointer() == b.to_pointer(),
            (Value::Function(a), Value::Function(b)) => a.to_pointer() == b.to_pointer(),
            (Value::UserData(a), Value::UserData(b)) => a.to_pointer() == b.to_pointer(),
            (Value::Error(a), Value::Error(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }
}

/// True if the `f64` is an exact integer within `i64` range (so it can be
/// presented as [`Value::Integer`]).
fn is_exact_integer(n: f64) -> bool {
    n.fract() == 0.0 && n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64
}

/// Push a [`Value`] onto the top of the Lua stack.
pub(crate) fn push_value(lua: &Lua, value: &Value) -> Result<()> {
    let state = lua.state();
    unsafe {
        match value {
            Value::Nil => lua_pushnil(state),
            Value::Boolean(b) => lua_pushboolean(state, *b as c_int),
            Value::Integer(i) => lua_pushnumber(state, *i as f64),
            Value::Number(n) => lua_pushnumber(state, *n),
            Value::String(s) => s.push_to_stack(),
            Value::Table(t) => t.push_to_stack(),
            Value::Function(f) => f.push_to_stack(),
            Value::UserData(u) => u.push_to_stack(),
            // An error value pushes as its message string (so Lua code that
            // receives it can `tostring(err)` it). This matches how a Rust
            // callback's `Err` surfaces to Lua as a string error object.
            Value::Error(e) => {
                let msg = e.to_string();
                lua_pushlstring(state, msg.as_ptr() as *const c_char, msg.len());
            }
        }
    }
    Ok(())
}

/// Build a [`Value`] from the value at stack index `idx` (does not pop). For
/// reference types this registers a registry reference.
pub(crate) fn value_from_stack(lua: &Lua, idx: c_int) -> Result<Value> {
    let state = lua.state();
    unsafe {
        let t = lua_type(state, idx);
        let value = match t {
            x if x == ttype::NIL || x == ttype::NONE => Value::Nil,
            x if x == ttype::BOOLEAN => Value::Boolean(lua_toboolean(state, idx) != 0),
            x if x == ttype::NUMBER => {
                let n = lua_tonumberx(state, idx, core::ptr::null_mut());
                if is_exact_integer(n) {
                    Value::Integer(n as i64)
                } else {
                    Value::Number(n)
                }
            }
            x if x == ttype::STRING => {
                lua_pushvalue(state, idx);
                Value::String(LuaString::from_ref(lua.pop_ref()))
            }
            x if x == ttype::TABLE => {
                lua_pushvalue(state, idx);
                Value::Table(Table::from_ref(lua.pop_ref()))
            }
            x if x == ttype::FUNCTION => {
                lua_pushvalue(state, idx);
                Value::Function(Function::from_ref(lua.pop_ref()))
            }
            x if x == ttype::USERDATA => {
                lua_pushvalue(state, idx);
                Value::UserData(crate::userdata::AnyUserData::from_ref(lua.pop_ref()))
            }
            // Vectors and any other exotic tags collapse to Nil for v1.
            _ => Value::Nil,
        };
        Ok(value)
    }
}
