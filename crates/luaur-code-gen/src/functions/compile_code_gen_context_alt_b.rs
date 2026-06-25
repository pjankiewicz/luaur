use crate::records::compilation_options::CompilationOptions;
use crate::records::compilation_result::CompilationResult;
use crate::records::compilation_stats::CompilationStats;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::module_id::ModuleId;
use core::ffi::c_int;

/// **Out of scope** — see `compile_code_gen_context.rs`. `Luau::CodeGen::compileInternal`
/// is the native-codegen entry point; luaur executes via the bytecode interpreter
/// (docs/CONFORMANCE.md). It was a phantom `extern` to the C++ symbol (unresolved
/// on Windows), so it is stubbed explicitly.
#[allow(unused_variables)]
pub fn compile_lua_state_i32_compilation_options_compilation_stats(
    l: *mut lua_State,
    idx: c_int,
    options: &CompilationOptions,
    stats: *mut CompilationStats,
) -> CompilationResult {
    let _ = ModuleId::default();
    unimplemented!(
        "luaur does not execute JIT-compiled native code (out of scope; see docs/CONFORMANCE.md)"
    )
}
