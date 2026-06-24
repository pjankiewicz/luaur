//! The [`LuaString`] handle. Mirrors `mlua::String`.

use std::rc::Rc;

use crate::error::{Error, Result};
use crate::ffi::*;
use crate::state::{Lua, LuaRef};

/// A garbage-collected Lua string.
///
/// Mirrors `mlua::String`. Holds a registry reference to the underlying Lua
/// string so the bytes stay alive for the handle's lifetime.
#[derive(Clone)]
pub struct LuaString {
    pub(crate) reference: Rc<LuaRef>,
}

impl LuaString {
    pub(crate) fn from_ref(reference: LuaRef) -> LuaString {
        LuaString {
            reference: Rc::new(reference),
        }
    }

    /// Push this string onto the owning state's stack.
    pub(crate) unsafe fn push_to_stack(&self) {
        self.reference.push();
    }

    /// Get the raw bytes of the string (a copy).
    ///
    /// Mirrors `mlua::String::as_bytes` (we return an owned `Vec<u8>` rather
    /// than a borrowed guard — a deliberate simplification).
    pub fn as_bytes(&self) -> Vec<u8> {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let mut len = 0usize;
            let p = lua_tolstring(state, -1, &mut len);
            let bytes = if p.is_null() {
                Vec::new()
            } else {
                core::slice::from_raw_parts(p as *const u8, len).to_vec()
            };
            lua_pop(state, 1);
            bytes
        }
    }

    /// Get the string as a UTF-8 `&str`, erroring if it is not valid UTF-8.
    ///
    /// Mirrors `mlua::String::to_str` (returns an owned `String` here).
    pub fn to_str(&self) -> Result<String> {
        let bytes = self.as_bytes();
        String::from_utf8(bytes).map_err(|e| {
            Error::FromLuaConversionError {
                from: "string",
                to: "String".to_string(),
                message: Some(format!("invalid utf-8: {e}")),
            }
        })
    }

    /// Get the string lossily as a Rust `String` (invalid UTF-8 replaced).
    ///
    /// Mirrors `mlua::String::to_string_lossy`.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.as_bytes()).into_owned()
    }

    /// The raw bytes with a trailing NUL appended (Lua strings are NUL
    /// terminated). Mirrors `mlua::String::as_bytes_with_nul`.
    pub fn as_bytes_with_nul(&self) -> Vec<u8> {
        let mut v = self.as_bytes();
        v.push(0);
        v
    }

    /// A raw pointer identifying the interned string (for identity
    /// comparison). Mirrors `mlua::String::to_pointer`.
    pub fn to_pointer(&self) -> *const std::ffi::c_void {
        let state = self.reference.state();
        unsafe {
            self.reference.push();
            let p = lua_topointer(state, -1);
            lua_pop(state, 1);
            p
        }
    }

    /// A `Display`-able view that renders the bytes lossily as UTF-8.
    /// Mirrors `mlua::String::display`.
    pub fn display(&self) -> LuaStringDisplay {
        LuaStringDisplay(self.to_string_lossy())
    }
}

/// `Display` adapter returned by [`LuaString::display`].
pub struct LuaStringDisplay(String);

impl std::fmt::Display for LuaStringDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Debug for LuaString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mirror mlua: valid utf-8 prints as a normal Rust string literal,
        // otherwise as a byte-string literal `b"..."`.
        let bytes = self.as_bytes();
        match std::str::from_utf8(&bytes) {
            Ok(s) => write!(f, "{s:?}"),
            Err(_) => {
                f.write_str("b\"")?;
                for &b in &bytes {
                    match b {
                        b'\0' => f.write_str("\\0")?,
                        b'\r' => f.write_str("\\r")?,
                        b'\n' => f.write_str("\\n")?,
                        b'\t' => f.write_str("\\t")?,
                        b'\\' => f.write_str("\\\\")?,
                        b'"' => f.write_str("\\\"")?,
                        0x20..=0x7e => f.write_str(&(b as char).to_string())?,
                        _ => write!(f, "\\x{b:02x}")?,
                    }
                }
                f.write_str("\"")
            }
        }
    }
}

impl PartialEq for LuaString {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for LuaString {}

impl PartialOrd for LuaString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LuaString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_bytes().cmp(&other.as_bytes())
    }
}

impl std::hash::Hash for LuaString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_bytes().hash(state);
    }
}

// --- Comparisons against common Rust byte/str types ------------------------

macro_rules! impl_str_eq {
    ($($ty:ty => $conv:expr),* $(,)?) => {$(
        impl PartialEq<$ty> for LuaString {
            fn eq(&self, other: &$ty) -> bool {
                let f: fn(&$ty) -> &[u8] = $conv;
                self.as_bytes() == f(other)
            }
        }
        impl PartialOrd<$ty> for LuaString {
            fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                let f: fn(&$ty) -> &[u8] = $conv;
                Some(self.as_bytes().as_slice().cmp(f(other)))
            }
        }
    )*};
}

impl_str_eq! {
    str => |s| s.as_bytes(),
    String => |s| s.as_bytes(),
    [u8] => |s| s,
    Vec<u8> => |s| s.as_slice(),
}

impl PartialEq<&str> for LuaString {
    fn eq(&self, other: &&str) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}
impl PartialOrd<&str> for LuaString {
    fn partial_cmp(&self, other: &&str) -> Option<std::cmp::Ordering> {
        Some(self.as_bytes().as_slice().cmp(other.as_bytes()))
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for LuaString {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.as_bytes() == other.as_slice()
    }
}
impl<const N: usize> PartialOrd<&[u8; N]> for LuaString {
    fn partial_cmp(&self, other: &&[u8; N]) -> Option<std::cmp::Ordering> {
        Some(self.as_bytes().as_slice().cmp(other.as_slice()))
    }
}

impl PartialEq<std::borrow::Cow<'_, [u8]>> for LuaString {
    fn eq(&self, other: &std::borrow::Cow<'_, [u8]>) -> bool {
        self.as_bytes() == other.as_ref()
    }
}

/// Helper to create a fresh Lua string from bytes on a given state, returning a
/// handle. Used by [`Lua::create_string`].
pub(crate) fn create_string(lua: &Lua, bytes: &[u8]) -> LuaString {
    let state = lua.state();
    unsafe {
        lua_pushlstring(state, bytes.as_ptr() as *const c_char, bytes.len());
        LuaString::from_ref(lua.pop_ref())
    }
}
