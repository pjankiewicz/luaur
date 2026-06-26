use luaur_ast::records::location::Location;

#[derive(Debug, Clone)]
pub struct InternalCompilerError {
    pub message: alloc::string::String,
    pub module_name: Option<alloc::string::String>,
    pub location: Option<Location>,
    /// A NUL-terminated copy of `message` for the C++-style `what()`, which
    /// returns `*const c_char` read with `CStr::from_ptr`. A Rust `String` is
    /// not NUL-terminated, so `message.as_ptr()` would over-read past the buffer
    /// (UB; flaky garbage across allocators). Built once at construction via
    /// [`InternalCompilerError::new`] so the pointer stays valid for `&self`.
    pub(crate) c_message: alloc::ffi::CString,
}

impl InternalCompilerError {
    /// Build an `InternalCompilerError`, materializing the NUL-terminated
    /// `what()` view from `message`.
    pub fn new(
        message: alloc::string::String,
        module_name: Option<alloc::string::String>,
        location: Option<Location>,
    ) -> Self {
        let c_message = nul_terminated(&message);
        Self {
            message,
            module_name,
            location,
            c_message,
        }
    }
}

/// NUL-terminated C string from `s`, stripping any (never-expected) interior
/// NULs so construction cannot fail even mid-panic.
pub(crate) fn nul_terminated(s: &str) -> alloc::ffi::CString {
    match alloc::ffi::CString::new(s) {
        Ok(c) => c,
        Err(_) => alloc::ffi::CString::new(s.replace('\0', "")).unwrap_or_default(),
    }
}

unsafe impl Send for InternalCompilerError {}
unsafe impl Sync for InternalCompilerError {}

#[cfg(feature = "std")]
impl std::error::Error for InternalCompilerError {}

impl core::fmt::Display for InternalCompilerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}
