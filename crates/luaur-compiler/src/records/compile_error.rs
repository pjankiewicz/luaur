extern crate alloc;

use alloc::string::String;
use luaur_ast::records::location::Location;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub(crate) location: Location,
    pub(crate) message: String,
    /// A NUL-terminated copy of `message` for the C++-style [`CompileError::what`],
    /// which returns `*const c_char` and is read by callers with `CStr::from_ptr`.
    ///
    /// A Rust `String` is **not** NUL-terminated, so handing out
    /// `message.as_ptr()` makes the reader over-run past the buffer into adjacent
    /// memory — undefined behavior that surfaces as flaky trailing garbage
    /// depending on the allocator / load (the cross-platform failure that
    /// followed issue #3). Materialized once at construction so the pointer stays
    /// valid for `&self`'s lifetime, mirroring C++'s `std::string::c_str()`.
    pub(crate) c_message: alloc::ffi::CString,
}

impl CompileError {
    /// Build a `CompileError`, materializing the NUL-terminated `what()` view.
    pub(crate) fn new(location: Location, message: String) -> CompileError {
        let c_message = nul_terminated(&message);
        CompileError {
            location,
            message,
            c_message,
        }
    }
}

/// Build a NUL-terminated C string from `s`. Compile-error messages never
/// contain interior NUL bytes; strip any defensively so construction (which may
/// run while a panic is being raised) can never itself fail.
pub(crate) fn nul_terminated(s: &str) -> alloc::ffi::CString {
    match alloc::ffi::CString::new(s) {
        Ok(c) => c,
        Err(_) => alloc::ffi::CString::new(s.replace('\0', "")).unwrap_or_default(),
    }
}

impl core::fmt::Display for CompileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}
