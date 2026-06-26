use crate::records::internal_compiler_error::InternalCompilerError;
use core::ffi::c_char;

impl InternalCompilerError {
    #[inline]
    pub fn what(&self) -> *const c_char {
        // NUL-terminated (see `c_message`): `message.as_ptr()` would over-read.
        self.c_message.as_ptr()
    }
}
