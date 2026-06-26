use crate::records::compile_error::CompileError;
use core::ffi::c_char;

impl CompileError {
    pub fn what(&self) -> *const c_char {
        // NUL-terminated (see `CompileError::c_message`): `message.as_ptr()`
        // would over-read in `CStr::from_ptr` since a Rust `String` has no NUL.
        self.c_message.as_ptr()
    }
}
