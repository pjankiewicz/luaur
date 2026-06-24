//! The [`Chunk`] builder returned by [`Lua::load`]. Mirrors `mlua::Chunk`.
//!
//! Compilation reuses the same machinery as the umbrella `luaur` crate's
//! `compile`/`eval` helpers: source -> `luaur_compiler::compile` -> bytecode ->
//! `luau_load` -> a Lua function on the stack -> `lua_pcall`.

use crate::error::{Error, Result};
use crate::ffi::*;
use crate::function::Function;
use crate::state::Lua;
use crate::traits::FromLuaMulti;

use luaur_ast::records::parse_options::ParseOptions;
use luaur_bytecode::records::bytecode_encoder::BytecodeEncoder;
use luaur_compiler::functions::compile::compile as compiler_compile;
use luaur_compiler::records::compile_options::CompileOptions;

/// No-op bytecode encoder (same as the umbrella crate's `NoopEncoder`).
struct NoopEncoder;
impl BytecodeEncoder for NoopEncoder {
    fn encode(&mut self, _data: &mut [u32]) {}
}

/// A not-yet-executed piece of Lua source.
///
/// Mirrors `mlua::Chunk`. Produced by [`Lua::load`]; finalized with
/// [`Chunk::exec`], [`Chunk::eval`], or [`Chunk::into_function`].
pub struct Chunk {
    pub(crate) lua: Lua,
    pub(crate) source: String,
    pub(crate) name: String,
}

impl Chunk {
    /// Override the chunk name shown in error messages / tracebacks.
    ///
    /// Mirrors `mlua::Chunk::set_name`.
    pub fn set_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// The chunk name used for error messages / tracebacks.
    ///
    /// Mirrors `mlua::Chunk::name`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Compile the source to bytecode (or return a [`Error::SyntaxError`]).
    fn compile(&self) -> Result<Vec<u8>> {
        let options = CompileOptions::default();
        let parse_options = ParseOptions::default();
        let mut encoder = NoopEncoder;
        let owned = self.source.clone();
        let blob = compiler_compile(
            &owned,
            &options,
            &parse_options,
            &mut encoder as *mut dyn BytecodeEncoder,
        );
        let bytes = blob.into_bytes();
        // A leading 0 byte is the compiler's error marker.
        if bytes.first() == Some(&0u8) {
            let message = String::from_utf8_lossy(&bytes[1..]).into_owned();
            return Err(Error::SyntaxError {
                message,
                incomplete_input: false,
            });
        }
        Ok(bytes)
    }

    /// Load the compiled chunk and leave the resulting function on top of the
    /// stack, returning a [`Function`] handle.
    ///
    /// Mirrors `mlua::Chunk::into_function`.
    pub fn into_function(self) -> Result<Function> {
        let bytecode = self.compile()?;
        let state = self.lua.state();
        unsafe {
            let chunkname = std::ffi::CString::new(format!("={}", self.name))
                .unwrap_or_else(|_| std::ffi::CString::new("=chunk").unwrap());
            let rc = luau_load(
                state,
                chunkname.as_ptr(),
                bytecode.as_ptr() as *const c_char,
                bytecode.len(),
                0,
            );
            if rc != 0 {
                // luau_load failure leaves an error message on the stack.
                return Err(self.lua.pop_error(rc));
            }
            Ok(Function::from_ref(self.lua.pop_ref()))
        }
    }

    /// Run the chunk for its side effects, discarding return values.
    ///
    /// Mirrors `mlua::Chunk::exec`.
    pub fn exec(self) -> Result<()> {
        let f = self.into_function()?;
        f.call::<()>(())
    }

    /// Run the chunk and convert its return value(s) to `R`.
    ///
    /// Mirrors `mlua::Chunk::eval`.
    pub fn eval<R: FromLuaMulti>(self) -> Result<R> {
        let f = self.into_function()?;
        f.call::<R>(())
    }
}
