use crate::records::compile_error::CompileError;
use luaur_ast::records::location::Location;

pub fn compile_error_compile_error(location: Location, message: String) -> CompileError {
    CompileError::new(location, message)
}
