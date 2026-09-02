//! `FromLua` / `IntoLua` / `FromLuaMulti` / `IntoLuaMulti` impls for the common
//! Rust types. Mirrors the impls in `mlua::conversion`.

use crate::error::{Error, Result};
use crate::function::Function;
use crate::multi::{MultiValue, Variadic};
use crate::state::Lua;
use crate::string::LuaString;
use crate::sys::{lua_tonumberx, lua_type, ttype};
use crate::table::Table;
use crate::traits::{FromLua, FromLuaMulti, IntoLua, IntoLuaMulti};
use crate::value::{Integer, Number, Value};
use core::ffi::c_int;

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
                // Convert an (already truncated, finite) float directly into the
                // target type with a range check against its f64 bounds, so a
                // value beyond `i64` range (e.g. `2^64` for `u64`) is correctly
                // rejected rather than silently saturating through `i64`.
                fn from_float(f: f64) -> Result<$ty> {
                    // Truncate toward zero, then range-check in `i128` space so a
                    // value beyond the target's range (e.g. `2^64` for `u64`) is
                    // rejected rather than saturating. Values whose magnitude
                    // exceeds `i128` are likewise out of range for any `$ty`.
                    let t = f.trunc();
                    if t < (i128::MIN as f64) || t > (i128::MAX as f64) {
                        return Err(Error::FromLuaConversionError {
                            from: "number",
                            to: stringify!($ty).to_string(),
                            message: Some("out of range".to_string()),
                        });
                    }
                    let wide = t as i128;
                    <$ty>::try_from(wide).map_err(|_| Error::FromLuaConversionError {
                        from: "number",
                        to: stringify!($ty).to_string(),
                        message: Some("out of range".to_string()),
                    })
                }
                match value {
                    Value::Integer(i) => {
                        <$ty>::try_from(i).map_err(|_| Error::FromLuaConversionError {
                            from: "number",
                            to: stringify!($ty).to_string(),
                            message: Some("out of range".to_string()),
                        })
                    }
                    Value::Number(f) => {
                        // Luau is f64-backed: `tonumber`/`lua_tointegerx`
                        // **truncate** a fractional float toward zero rather than
                        // rejecting it (matching mlua on the Luau backend). A
                        // non-finite number has no integer representation.
                        if !f.is_finite() {
                            return Err(Error::FromLuaConversionError {
                                from: "number",
                                to: stringify!($ty).to_string(),
                                message: Some("number has no integer representation".to_string()),
                            });
                        }
                        from_float(f)
                    }
                    // Lua coerces numeric strings to numbers (mirrors mlua's
                    // string fallback for integer conversions).
                    Value::String(ref s) => {
                        let text = s.to_string_lossy();
                        let trimmed = text.trim();
                        if let Ok(i) = trimmed.parse::<i64>() {
                            <$ty>::try_from(i).map_err(|_| Error::FromLuaConversionError {
                                from: "string",
                                to: stringify!($ty).to_string(),
                                message: Some("out of range".to_string()),
                            })
                        } else if let Ok(f) = trimmed.parse::<f64>() {
                            if !f.is_finite() {
                                return Err(Error::FromLuaConversionError {
                                    from: "string",
                                    to: stringify!($ty).to_string(),
                                    message: Some("number has no integer representation".to_string()),
                                });
                            }
                            from_float(f)
                        } else {
                            Err(Error::FromLuaConversionError {
                                from: "string",
                                to: stringify!($ty).to_string(),
                                message: Some("not a number".to_string()),
                            })
                        }
                    }
                    other => {
                        Err(Error::FromLuaConversionError {
                            from: other.type_name(),
                            to: stringify!($ty).to_string(),
                            message: None,
                        })
                    }
                }
            }
        }
    )*};
}

impl_integer!(i8, u8, i16, u16, i32, u32, i64, u64, isize, usize);

// 128-bit integers exceed Luau's f64-backed number range, so they round-trip
// **lossily** through `f64` (matching mlua, which has no 128-bit Lua number).
// Values within f64's exactly-representable range survive the round trip.
macro_rules! impl_integer_128 {
    ($($ty:ty),*) => {$(
        impl IntoLua for $ty {
            fn into_lua(self, _lua: &Lua) -> Result<Value> {
                Ok(Value::Number(self as f64))
            }
        }

        impl FromLua for $ty {
            fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
                let n: f64 = match value {
                    Value::Integer(i) => i as f64,
                    Value::Number(f) => f,
                    Value::String(ref s) => {
                        s.to_string_lossy().trim().parse::<f64>().map_err(|_| {
                            Error::FromLuaConversionError {
                                from: "string",
                                to: stringify!($ty).to_string(),
                                message: Some("not a number".to_string()),
                            }
                        })?
                    }
                    other => {
                        return Err(Error::FromLuaConversionError {
                            from: other.type_name(),
                            to: stringify!($ty).to_string(),
                            message: None,
                        });
                    }
                };
                if n.fract() != 0.0 || !n.is_finite() {
                    return Err(Error::FromLuaConversionError {
                        from: "number",
                        to: stringify!($ty).to_string(),
                        message: Some("number has no integer representation".to_string()),
                    });
                }
                Ok(n as $ty)
            }
        }
    )*};
}

impl_integer_128!(i128, u128);

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

impl IntoLua for f32 {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Number(self as Number))
    }
}

// Mirrors mlua's `lua_convert_float!`: the value-level `from_lua` keeps the
// pre-existing behavior (numeric strings coerce, integer values widen, exact
// error messages). The added stack fast path (`from_stack`) reads the raw
// `lua_Number` straight off the stack, bypassing the `Value` representation,
// like mlua does — this preserves details `Value` folds away (notably the
// sign bit of `-0.0`, which `value_from_stack`'s whole-number normalization
// to `Value::Integer` would drop).
macro_rules! lua_convert_float {
    ($x:ty, $to:literal) => {
        impl FromLua for $x {
            fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
                match value {
                    Value::Number(n) => Ok(n as $x),
                    Value::Integer(i) => Ok(i as f64 as $x),
                    // Lua coerces numeric strings to numbers (mirrors mlua).
                    Value::String(ref s) => {
                        let text = s.to_string_lossy();
                        text.trim()
                            .parse::<$x>()
                            .or_else(|_| text.trim().parse::<f64>().map(|n| n as $x))
                            .map_err(|_| Error::FromLuaConversionError {
                                from: "string",
                                to: $to.to_string(),
                                message: Some("not a number".to_string()),
                            })
                    }
                    other => Err(Error::FromLuaConversionError {
                        from: other.type_name(),
                        to: $to.to_string(),
                        message: None,
                    }),
                }
            }

            unsafe fn from_stack(idx: c_int, lua: &Lua) -> Result<Self> {
                let state = lua.state();
                unsafe {
                    if lua_type(state, idx) == ttype::NUMBER {
                        // `lua_tonumberx` with a null `isnum` out-param: the
                        // conversion cannot fail for an actual number.
                        return Ok(lua_tonumberx(state, idx, core::ptr::null_mut()) as $x);
                    }
                }
                // Fall back to the value-level conversion (string coercion
                // and error messages stay identical to the slow path).
                Self::from_lua(lua.value_from_stack(idx)?, lua)
            }
        }
    };
}
lua_convert_float!(f64, "f64");
lua_convert_float!(f32, "f32");

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

impl IntoLua for &LuaString {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::String(self.clone()))
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

impl IntoLua for &Table {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Table(self.clone()))
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

impl IntoLua for &Function {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Function(self.clone()))
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
// Vector (Luau) <-> Value::Vector
// ---------------------------------------------------------------------------

impl IntoLua for crate::Vector {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Vector(self))
    }
}

impl FromLua for crate::Vector {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Vector(v) => Ok(v),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "vector".to_string(),
                message: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Buffer (Luau) <-> Value::Buffer
// ---------------------------------------------------------------------------

impl IntoLua for crate::Buffer {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Buffer(self))
    }
}

impl IntoLua for &crate::Buffer {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::Buffer(self.clone()))
    }
}

impl FromLua for crate::Buffer {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::Buffer(buf) => Ok(buf),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "buffer".to_string(),
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
        // A Luau vector converts to an array of its components (matching mlua:
        // `let v: [f64; 3] = lua.load("vector.create(1,2,3)").eval()?`).
        if let Value::Vector(v) = value {
            if N == crate::Vector::SIZE {
                let x = T::from_lua(Value::Number(v.x() as f64), lua)?;
                let y = T::from_lua(Value::Number(v.y() as f64), lua)?;
                let z = T::from_lua(Value::Number(v.z() as f64), lua)?;
                let comps = vec![x, y, z];
                return <[T; N]>::try_from(comps).map_err(|_| Error::FromLuaConversionError {
                    from: "vector",
                    to: format!("[T; {N}]"),
                    message: None,
                });
            }
            return Err(Error::FromLuaConversionError {
                from: "vector",
                to: format!("[T; {N}]"),
                message: Some(format!(
                    "expected array of length {}, got {N}",
                    crate::Vector::SIZE
                )),
            });
        }
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
                let keys: Vec<Result<T>> =
                    t.pairs::<T, Value>().map(|p| p.map(|(k, _)| k)).collect();
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

impl IntoLua for crate::light_userdata::LightUserData {
    fn into_lua(self, _lua: &Lua) -> Result<Value> {
        Ok(Value::LightUserData(self))
    }
}

impl FromLua for crate::light_userdata::LightUserData {
    fn from_lua(value: Value, _lua: &Lua) -> Result<Self> {
        match value {
            Value::LightUserData(lud) => Ok(lud),
            other => Err(Error::FromLuaConversionError {
                from: other.type_name(),
                to: "LightUserData".to_string(),
                message: None,
            }),
        }
    }
}

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

// `IntoLuaMulti` for tuples: each element may itself spread to multiple values
// (e.g. a trailing `Variadic<T>`), so concatenate their `MultiValue`s.
macro_rules! impl_tuple_into {
    () => {};
    ($first:ident $($rest:ident)*) => {
        impl_tuple_into!($($rest)*);

        #[allow(non_snake_case)]
        impl<$first: IntoLuaMulti, $($rest: IntoLuaMulti,)*> IntoLuaMulti for ($first, $($rest,)*) {
            fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue> {
                let ($first, $($rest,)*) = self;
                let mut m = MultiValue::new();
                for v in $first.into_lua_multi(lua)? { m.push_back(v); }
                $( for v in $rest.into_lua_multi(lua)? { m.push_back(v); } )*
                Ok(m)
            }
        }
    };
}

impl_tuple_into!(A B C D E F G H I J K L);

// `FromLuaMulti` for tuples. The **last** element is parsed as a `FromLuaMulti`
// (so it may be a trailing `Variadic<T>` that consumes every remaining value);
// the preceding elements each consume exactly one value via `FromLua`.
//
// Because every `FromLua` type is also `FromLuaMulti` (the blanket impl), a
// single impl per arity with the last slot bound `FromLuaMulti` subsumes the
// all-`FromLua` case as well — no overlapping impls. Mirrors mlua's tuple
// `FromLuaMulti`, which lets the final slot soak up the rest.
macro_rules! impl_tuple_from {
    // Base: a 1-tuple is just its single `FromLuaMulti` element.
    ($last:ident;) => {
        #[allow(non_snake_case)]
        impl<$last: FromLuaMulti> FromLuaMulti for ($last,) {
            fn from_lua_multi(values: MultiValue, lua: &Lua) -> Result<Self> {
                Ok(($last::from_lua_multi(values, lua)?,))
            }
        }
    };
    ($last:ident; $head0:ident $($head:ident)*) => {
        impl_tuple_from!($last; $($head)*);

        #[allow(non_snake_case)]
        impl<$head0: FromLua, $($head: FromLua,)* $last: FromLuaMulti>
            FromLuaMulti for ($head0, $($head,)* $last,)
        {
            fn from_lua_multi(mut values: MultiValue, lua: &Lua) -> Result<Self> {
                let $head0 = $head0::from_lua(values.pop_front().unwrap_or(Value::Nil), lua)?;
                $( let $head = $head::from_lua(values.pop_front().unwrap_or(Value::Nil), lua)?; )*
                let $last = $last::from_lua_multi(values, lua)?;
                Ok(($head0, $($head,)* $last,))
            }
        }
    };
}

// Arities 1..=12 (last element `FromLuaMulti`, the rest `FromLua`).
impl_tuple_from!(L; A B C D E F G H I J K);
