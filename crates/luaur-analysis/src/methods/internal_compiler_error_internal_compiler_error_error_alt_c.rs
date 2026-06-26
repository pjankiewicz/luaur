use crate::records::internal_compiler_error::InternalCompilerError;
use luaur_ast::records::location::Location;

impl InternalCompilerError {
    pub fn internal_compiler_error_string_string_location(
        message: alloc::string::String,
        module_name: alloc::string::String,
        location: Location,
    ) -> Self {
        Self::new(message, Some(module_name), Some(location))
    }
}
