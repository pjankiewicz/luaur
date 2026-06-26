use crate::records::internal_compiler_error::InternalCompilerError;
use crate::records::recursion_limit_exception::RecursionLimitException;
use alloc::format;
use alloc::string::String;

impl RecursionLimitException {
    pub fn new(system: &str) -> Self {
        Self {
            base: InternalCompilerError::new(
                format!("Internal recursion counter limit exceeded in {}", system),
                None,
                None,
            ),
        }
    }
}
