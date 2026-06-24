//! `FromLua` / `IntoLua` / `FromLuaMulti` / `IntoLuaMulti` impls for the common
//! Rust types. Mirrors the impls in `mlua::conversion`.

use crate::error::{Error, Result};
use crate::function::Function;
use crate::multi::{MultiValue, Variadic};
use crate::state::Lua;
use crate::string::LuaString;
use crate::table::Table;
use crate::traits::{FromLua, FromLuaMulti, IntoLua, IntoLuaMulti};
use crate::value::{Integer, Number, Value};

// ---------------------------------------------------------------------------
// Value itself
// ---------------------------------------------------------------------------

impl IntoLua for Value {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(self)
    }
}

impl FromLua for Value {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        Ok(value)
    }
}

// ---------------------------------------------------------------------------
// Unit / nil
// ---------------------------------------------------------------------------
//
// NOTE: `()` is deliberately NOT a single-value (`IntoLua`/`FromLua`) type.
// In Lua, `()` means *zero* values, not one nil — so it implements only the
// multi-value traits below (producing/consuming no stack values). This also
// avoids a coherence clash with the blanket `impl<T: IntoLua> IntoLuaMulti`.

// `()` as a *multi* value means "no values" in both directions.
impl IntoLuaMulti for () {
    fn into_lua_multi(self, _lua: &Lua) -> Result<MultiValue> {
        Ok(MultiValue::new())
    }
}

impl FromLuaMulti for () {
    fn from_lua_multi(_values: MultiValue, _lua: &Lua) -> Result<Self> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// bool
// ---------------------------------------------------------------------------

impl IntoLua for bool {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Boolean(self))
    }
}

impl FromLua for bool {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        // Lua truthiness: nil and false are false, everything else is true.
        Ok(match value {
            Value::Nil => false,
            Value::Boolean(b) => b,
            _ => true,
        })
    }
}

// ---------------------------------------------------------------------------
// Integers (range-checked)
// ---------------------------------------------------------------------------

macro_rules! impl_integer {
    ($($ty:ty),*) => {$(
        impl IntoLua for $ty {
            fn into_lua(self, _lua: &Lua) -> Result<Value> {
                let as_i64 = i64::try_from(self).map_err(|_| Error::ToLuaConversionError {
                    from: stringify!($ty),
                    to: "integer",
                    message: Some("value out of i64 range".to_string()),
                })?;
                Ok(Value::Integer(as_i64))
            }
        }

        impl FromLua for $ty {
            fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
                let n: i64 = match value {
                    Value::Integer(i) => i,
                    Value::Number(f) => {
                        if f.fract() != 0.0 || !f.is_finite() {
                            return Err(Error::FromLuaConversionError {
                                from: "number",
                                to: stringify!($ty).to_string(),
                                message: Some("number has no integer representation".to_string()),
                            });
                        }
                        f as i64
                    }
                    // Lua coerces numeric strings to numbers (mirrors mlua's
                    // string fallback for integer conversions).
                    Value::String(ref s) => {
                        let text = s.to_string_lossy();
                        let trimmed = text.trim();
                        if let Ok(i) = trimmed.parse::<i64>() {
                            i
                        } else if let Ok(f) = trimmed.parse::<f64>() {
                            if f.fract() != 0.0 || !f.is_finite() {
                                return Err(Error::FromLuaConversionError {
                                    from: "string",
                                    to: stringify!($ty).to_string(),
                                    message: Some("number has no integer representation".to_string()),
                                });
                            }
                            f as i64
                        } else {
                            return Err(Error::FromLuaConversionError {
                                from: "string",
                                to: stringify!($ty).to_string(),
                                message: Some("not a number".to_string()),
                            });
                        }
                    }
                    other => {
                        return Err(Error::FromLuaConversionError {
                            from: other.type_name(),
                            to: stringify!($ty).to_string(),
                            message: None,
                        });
                    }
                };
                <$ty>::try_from(n).map_err(|_| Error::FromLuaConversionError {
                    from: "number",
                    to: stringify!($ty).to_string(),
                    message: Some("out of range".to_string()),
                })
            }
        }
    )*};
}

impl_integer!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

// `Integer` is `i64`, already covered by impl_integer.
const _: () = {
    // Compile-time assertion that Integer == i64.
    fn _assert(_x: Integer) -> i64 {
        _x
    }
};

// ---------------------------------------------------------------------------
// Floats
// ---------------------------------------------------------------------------

impl IntoLua for f64 {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Number(self))
    }
}

impl FromLua for f64 {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Number(n) => Ok(n),
            Value::Integer(i) => Ok(i as f64),
            // Lua coerces numeric strings to numbers (mirrors mlua).
            Value::String(ref s) => {
                let text = s.to_string_lossy();
                text.trim()
                    .parse::<f64>()
                    .map_err(|_| Error::FromLuaConversionError {
                        from: "string",
                        to: "f64".to_string(),
                        message: Some("not a number".to_string()),
                    })
            }
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "f64".to_string(),
                message: None,
            }),
        }
    }
}

impl IntoLua for f32 {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Number(self as Number))
    }
}

impl FromLua for f32 {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(f64::from_lua(value, lua)? as f32)
    }
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

impl IntoLua for String {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&self)))
    }
}

impl IntoLua for &str {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)))
    }
}

impl IntoLua for &String {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self)))
    }
}

impl FromLua for String {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::String(s) => s.to_str(),
            // Lua coerces numbers to strings in many contexts; mirror that.
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) => Ok(n.to_string()),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "String".to_string(),
                message: None,
            }),
        }
    }
}

impl IntoLua for LuaString {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::String(self))
    }
}

impl FromLua for LuaString {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::String(s) => Ok(s),
            Value::Integer(i) => Ok(_lua.create_string(i.to_string())),
            Value::Number(n) => Ok(_lua.create_string(n.to_string())),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "String".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Handles (Table, Function)
// ---------------------------------------------------------------------------

impl IntoLua for Table {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Table(self))
    }
}

impl FromLua for Table {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Table(t) => Ok(t),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "Table".to_string(),
                message: None,
            }),
        }
    }
}

impl IntoLua for Function {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Function(self))
    }
}

impl FromLua for Function {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Function(f) => Ok(f),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "Function".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Option<T>
// ---------------------------------------------------------------------------

impl<T: IntoLua> IntoLua for Option<T> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        match self {
            Some(v) => v.into_lua(lua),
            None => Ok(Value::Nil),
        }
    }
}

impl<T: FromLua> FromLua for Option<T> {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        match value {
            Value::Nil => Ok(None),
            other => Ok(Some(T::from_lua(other, lua)?)),
        }
    }
}

// ---------------------------------------------------------------------------
// Vec<T> <-> sequence table
// ---------------------------------------------------------------------------

impl<T: IntoLua> IntoLua for Vec<T> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for (i, item) in self.into_iter().enumerate() {
            // Lua sequences are 1-based.
            table.set((i + 1) as i64, item)?;
        }
        Ok(Value::Table(table))
    }
}

impl<T: IntoLua + Clone> IntoLua for &[T] {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for (i, item) in self.iter().enumerate() {
            table.raw_set((i + 1) as i64, item.clone())?;
        }
        Ok(Value::Table(table))
    }
}

impl<T: FromLua> FromLua for Vec<T> {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Table(t) => {
                let len = t.raw_len();
                let mut out = Vec::with_capacity(len);
                for i in 1..=len {
                    out.push(t.raw_get::<T>(i as i64)?);
                }
                Ok(out)
            }
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "Vec".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Fixed-size arrays [T; N] <-> sequence table
// ---------------------------------------------------------------------------

impl<T: IntoLua, const N: usize> IntoLua for [T; N] {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for (i, item) in self.into_iter().enumerate() {
            table.raw_set((i + 1) as i64, item)?;
        }
        Ok(Value::Table(table))
    }
}

impl<T: FromLua, const N: usize> FromLua for [T; N] {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        let vec: Vec<T> = Vec::from_lua(value, lua)?;
        let len = vec.len();
        <[T; N]>::try_from(vec).map_err(|_| Error::FromLuaConversionError {
            from: "table",
            to: format!("[T; {N}]"),
            message: Some(format!("expected table of length {N}, got {len}")),
        })
    }
}

// ---------------------------------------------------------------------------
// HashMap / BTreeMap <-> table
// ---------------------------------------------------------------------------

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

impl<K: IntoLua + Eq + Hash, V: IntoLua, S: std::hash::BuildHasher> IntoLua for HashMap<K, V, S> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for (k, v) in self {
            table.raw_set(k, v)?;
        }
        Ok(Value::Table(table))
    }
}

impl<K: FromLua + Eq + Hash, V: FromLua, S: std::hash::BuildHasher + Default> FromLua
    for HashMap<K, V, S>
{
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Table(t) => {
                let mut out = HashMap::with_hasher(S::default());
                for pair in t.pairs::<K, V>() {
                    let (k, v) = pair?;
                    out.insert(k, v);
                }
                Ok(out)
            }
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "HashMap".to_string(),
                message: None,
            }),
        }
    }
}

impl<K: IntoLua + Ord, V: IntoLua> IntoLua for BTreeMap<K, V> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for (k, v) in self {
            table.raw_set(k, v)?;
        }
        Ok(Value::Table(table))
    }
}

impl<K: FromLua + Ord, V: FromLua> FromLua for BTreeMap<K, V> {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Table(t) => {
                let mut out = BTreeMap::new();
                for pair in t.pairs::<K, V>() {
                    let (k, v) = pair?;
                    out.insert(k, v);
                }
                Ok(out)
            }
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "BTreeMap".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// HashSet / BTreeSet <-> table (values become keys mapped to `true`)
// ---------------------------------------------------------------------------

impl<T: IntoLua + Eq + Hash, S: std::hash::BuildHasher> IntoLua for HashSet<T, S> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for item in self {
            table.raw_set(item, true)?;
        }
        Ok(Value::Table(table))
    }
}

impl<T: FromLua + Eq + Hash, S: std::hash::BuildHasher + Default> FromLua for HashSet<T, S> {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        from_lua_set(value, lua, "HashSet", |it| {
            let mut out = HashSet::with_hasher(S::default());
            for v in it {
                out.insert(v?);
            }
            Ok(out)
        })
    }
}

impl<T: IntoLua + Ord> IntoLua for BTreeSet<T> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        let table = lua.create_table();
        for item in self {
            table.raw_set(item, true)?;
        }
        Ok(Value::Table(table))
    }
}

impl<T: FromLua + Ord> FromLua for BTreeSet<T> {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        from_lua_set(value, lua, "BTreeSet", |it| {
            let mut out = BTreeSet::new();
            for v in it {
                out.insert(v?);
            }
            Ok(out)
        })
    }
}

/// A Lua table can represent a set in two ways (matching mlua): as a sequence
/// of values `{a, b, c}`, or as a map of keys `{[a] = true, ...}`. We support
/// both: if the table has a non-empty sequence part, take its values;
/// otherwise take its keys.
fn from_lua_set<T: FromLua, C>(
    value: Value,
    _lua: &Lua,
    to: &'static str,
    build: impl FnOnce(SetIter<T>) -> Result<C>,
) -> Result<C> {
    match value {
        Value::Table(t) => {
            if t.raw_len() > 0 {
                build(SetIter::Seq(t.sequence_values::<T>()))
            } else {
                let keys: Vec<Result<T>> = t.pairs::<T, Value>().map(|p| p.map(|(k, _)| k)).collect();
                build(SetIter::Keys(keys.into_iter()))
            }
        }
        other => Err(Error::FromLuaConversionError {
            from: other.type_name(),
            to: to.to_string(),
            message: None,
        }),
    }
}

enum SetIter<T: FromLua> {
    Seq(crate::table::TableSequence<T>),
    Keys(std::vec::IntoIter<Result<T>>),
}

impl<T: FromLua> Iterator for SetIter<T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SetIter::Seq(s) => s.next(),
            SetIter::Keys(k) => k.next(),
        }
    }
}

// ---------------------------------------------------------------------------
// char
// ---------------------------------------------------------------------------

impl IntoLua for char {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.to_string())))
    }
}

impl FromLua for char {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        match value {
            Value::String(_) | Value::Integer(_) | Value::Number(_) => {}
            other => {
                return Err(Error::FromLuaConversionError {
                    from: other.type_name(),
                    to: "char".to_string(),
                    message: Some("expected string or integer".to_string()),
                })
            }
        }
        if let Value::Integer(_) | Value::Number(_) = value {
            let i = i64::from_lua(value, lua)?;
            let cp = u32::try_from(i).ok().and_then(char::from_u32);
            return cp.ok_or(Error::FromLuaConversionError {
                from: "number",
                to: "char".to_string(),
                message: Some("integer out of range for a unicode char".to_string()),
            });
        }
        let s = String::from_lua(value, lua)?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => Ok(c),
            _ => Err(Error::FromLuaConversionError {
                from: "string",
                to: "char".to_string(),
                message: Some("expected string to have exactly one char".to_string()),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Cow<str>, Box<str>, CString
// ---------------------------------------------------------------------------

impl IntoLua for std::borrow::Cow<'_, str> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_ref())))
    }
}

impl IntoLua for Box<str> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(&*self)))
    }
}

impl FromLua for Box<str> {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(String::from_lua(value, lua)?.into_boxed_str())
    }
}

impl IntoLua for std::ffi::CString {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        Ok(Value::String(lua.create_string(self.as_bytes())))
    }
}

impl FromLua for std::ffi::CString {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        let bytes = match value {
            Value::String(s) => s.as_bytes(),
            other => {
                return Err(Error::FromLuaConversionError {
                    from: other.type_name(),
                    to: "CString".to_string(),
                    message: None,
                })
            }
        };
        std::ffi::CString::new(bytes).map_err(|e| Error::FromLuaConversionError {
            from: "string",
            to: "CString".to_string(),
            message: Some(format!("interior nul byte: {e}")),
        })
    }
}

impl<T: IntoLua> IntoLua for Box<[T]> {
    fn into_lua(self, lua: &Lua) -> Result<Value> {
        self.into_vec().into_lua(lua)
    }
}

impl<T: FromLua> FromLua for Box<[T]> {
    fn from_lua(value: Value, lua: &Lua) -> Result<Self> {
        Ok(Vec::<T>::from_lua(value, lua)?.into_boxed_slice())
    }
}

// ---------------------------------------------------------------------------
// Variadic<T>
// ---------------------------------------------------------------------------

impl<T: IntoLua> IntoLuaMulti for Variadic<T> {
    fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue> {
        let vec: Vec<T> = self.into();
        let mut m = MultiValue::with_capacity(vec.len());
        for item in vec {
            m.push_back(item.into_lua(lua)?);
        }
        Ok(m)
    }
}

impl<T: FromLua> FromLuaMulti for Variadic<T> {
    fn from_lua_multi(values: MultiValue, lua: &Lua) -> Result<Self> {
        let mut out = Vec::with_capacity(values.len());
        for v in values {
            out.push(T::from_lua(v, lua)?);
        }
        Ok(Variadic::from(out))
    }
}

// ---------------------------------------------------------------------------
// Error <-> Value::Error  +  Result<T, E> : IntoLuaMulti
// ---------------------------------------------------------------------------

impl IntoLua for Error {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Error(Box::new(self)))
    }
}

impl FromLua for Error {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        Ok(match value {
            Value::Error(e) => *e,
            // Any other Lua value converts to a runtime error carrying its
            // string form (mirrors mlua's `convert::<Error>`).
            Value::String(s) => Error::RuntimeError(s.to_string_lossy()),
            other => Error::RuntimeError(other.to_string().unwrap_or_default()),
        })
    }
}

/// `Result<T, E>` spreads as the success values on `Ok`, or as `(nil, error)`
/// on `Err` — mirroring mlua's `IntoLuaMulti for Result`.
impl<T: IntoLuaMulti, E: IntoLua> IntoLuaMulti for std::result::Result<T, E> {
    fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue> {
        match self {
            Ok(v) => v.into_lua_multi(lua),
            Err(e) => {
                let mut m = MultiValue::with_capacity(2);
                m.push_back(Value::Nil);
                m.push_back(e.into_lua(lua)?);
                Ok(m)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MultiValue passthrough
// ---------------------------------------------------------------------------

impl IntoLuaMulti for MultiValue {
    fn into_lua_multi(self, _lua: &Lua) -> Result<MultiValue> {
        Ok(self)
    }
}

impl FromLuaMulti for MultiValue {
    fn from_lua_multi(values: MultiValue, _lua: &Lua) -> Result<Self> {
        Ok(values)
    }
}

// ---------------------------------------------------------------------------
// Tuples (IntoLuaMulti / FromLuaMulti) up to 12
// ---------------------------------------------------------------------------

macro_rules! impl_tuple {
    () => {};
    ($first:ident $($rest:ident)*) => {
        impl_tuple!($($rest)*);

        #[allow(non_snake_case)]
        impl<$first: IntoLua, $($rest: IntoLua,)*> IntoLuaMulti for ($first, $($rest,)*) {
            fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue> {
                let ($first, $($rest,)*) = self;
                let mut m = MultiValue::new();
                m.push_back($first.into_lua(lua)?);
                $( m.push_back($rest.into_lua(lua)?); )*
                Ok(m)
            }
        }

        #[allow(non_snake_case)]
        impl<$first: FromLua, $($rest: FromLua,)*> FromLuaMulti for ($first, $($rest,)*) {
            fn from_lua_multi(mut values: MultiValue, lua: &Lua) -> Result<Self> {
                let $first = $first::from_lua(values.pop_front().unwrap_or(Value::Nil), lua)?;
                $( let $rest = $rest::from_lua(values.pop_front().unwrap_or(Value::Nil), lua)?; )*
                Ok(($first, $($rest,)*))
            }
        }
    };
}

impl_tuple!(A B C D E F G H I J K L);
