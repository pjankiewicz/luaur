//! Source: `Compiler/include/Luau/Compiler.h:84` + Compiler.cpp (hand-ported)
//! C varargs printf-style raise -> core::fmt::Arguments (project-wide precedent).

use crate::records::compile_error::CompileError;
use luaur_ast::records::location::Location;

impl CompileError {
    /// C++ `static LUAU_NORETURN void raise(const Location&, const char* format, ...)`
    /// Callers pass `format_args!(...)` (the varargs convention).
    pub fn raise(location: &Location, args: core::fmt::Arguments<'_>) -> ! {
        std::panic::panic_any(CompileError::new(*location, alloc::fmt::format(args)))
    }
}

/// Free-fn spelling some earlier translations import.
pub fn compile_error_raise(location: Location, args: core::fmt::Arguments<'_>) -> ! {
    CompileError::raise(&location, args)
}
