//! The conversion traits. Mirror `mlua::{IntoLua, FromLua, IntoLuaMulti,
//! FromLuaMulti}`.
//!
//! Like mlua, the traits also carry **internal, stack-oriented** hooks
//! (`from_stack`, `from_stack_multi`) used by the call/eval plumbing to skip
//! the intermediate [`Value`] representation for hot types (see
//! `lua_convert_float!` in mlua: `f64`/`f32` read the raw `lua_Number`
//! straight off the stack, which also preserves details `Value` folds away,
//! e.g. the sign bit of `-0.0`). These are `#[doc(hidden)]`: the public
//! surface is the value-level methods, whose *names and shapes*
//! (`into_lua(self, &Lua)`, `from_lua(value, &Lua)`, `into_lua_multi`,
//! `from_lua_multi`) match mlua exactly.
//!
//! Note these hooks take `&Lua` rather than mlua's internal `&RawLua`: luaur
//! has no separate raw handle.

use crate::error::Result;
use crate::multi::MultiValue;
use crate::state::Lua;
use crate::value::Value;
use core::ffi::c_int;

/// Convert a Rust value into a single Lua [`Value`].
///
/// Mirrors `mlua::IntoLua`.
pub trait IntoLua: Sized {
    /// Perform the conversion.
    fn into_lua(self, lua: &Lua) -> Result<Value>;
}

/// Convert a single Lua [`Value`] into a Rust value.
///
/// Mirrors `mlua::FromLua`.
pub trait FromLua: Sized {
    /// Perform the conversion.
    fn from_lua(value: Value, lua: &Lua) -> Result<Self>;

    /// Convert an argument at 1-based position `i`. The default forwards to
    /// [`FromLua::from_lua`]; specific impls can produce nicer messages.
    /// Mirrors `mlua::FromLua::from_lua_arg`.
    fn from_lua_arg(arg: Value, _i: usize, _to: Option<&str>, lua: &Lua) -> Result<Self> {
        Self::from_lua(arg, lua)
    }

    /// Internal: convert the value on the stack at index `idx` directly, the
    /// way mlua's `FromLua::from_stack` does. The default materializes a
    /// [`Value`] first (mirroring mlua's `lua.stack_value(idx, None)`);
    /// impls with a stack fast path (floats) read the raw value instead.
    /// Must not permanently mutate the stack (a temporary push is fine as
    /// long as it is popped).
    #[doc(hidden)]
    #[inline]
    unsafe fn from_stack(idx: c_int, lua: &Lua) -> Result<Self> {
        Self::from_lua(lua.value_from_stack(idx)?, lua)
    }
}

/// Convert a Rust value into a sequence of Lua values (multiple returns / args).
///
/// Mirrors `mlua::IntoLuaMulti`.
pub trait IntoLuaMulti: Sized {
    /// Perform the conversion.
    fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue>;
}

/// Convert a sequence of Lua values into a Rust value.
///
/// Mirrors `mlua::FromLuaMulti`.
pub trait FromLuaMulti: Sized {
    /// Perform the conversion.
    fn from_lua_multi(values: MultiValue, lua: &Lua) -> Result<Self>;

    /// Internal: collect `nvals` results sitting on the stack at indices
    /// `base+1 ..= base+nvals` directly. The default materializes each into
    /// a [`Value`] and forwards to [`FromLuaMulti::from_lua_multi`]
    /// (mirroring mlua's `FromLuaMulti::from_stack_multi` default).
    /// Must not permanently mutate the stack.
    #[doc(hidden)]
    #[allow(unused_variables)]
    unsafe fn from_stack_multi(base: c_int, nvals: c_int, lua: &Lua) -> Result<Self> {
        let mut values = MultiValue::with_capacity(nvals.max(0) as usize);
        for i in 0..nvals {
            values.push_back(lua.value_from_stack(base + 1 + i)?);
        }
        Self::from_lua_multi(values, lua)
    }
}

// Any single-value type is trivially a multi-value of length one.
impl<T: IntoLua> IntoLuaMulti for T {
    fn into_lua_multi(self, lua: &Lua) -> Result<MultiValue> {
        let mut m = MultiValue::with_capacity(1);
        m.push_back(self.into_lua(lua)?);
        Ok(m)
    }
}

// And any single-value type can be parsed from the first of a multi-value
// (extra values are ignored, matching Lua's "take what you need" calling
// convention). The stack-multi default forwards to `from_stack` so types
// with a stack fast path (floats) keep it on the multi-value path too.
impl<T: FromLua> FromLuaMulti for T {
    fn from_lua_multi(mut values: MultiValue, lua: &Lua) -> Result<Self> {
        let v = values.pop_front().unwrap_or(Value::Nil);
        T::from_lua(v, lua)
    }

    unsafe fn from_stack_multi(base: c_int, nvals: c_int, lua: &Lua) -> Result<Self> {
        if nvals <= 0 {
            return T::from_lua(Value::Nil, lua);
        }
        unsafe { T::from_stack(base + 1, lua) }
    }
}
