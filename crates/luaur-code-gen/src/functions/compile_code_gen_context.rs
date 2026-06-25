use crate::records::compilation_options::CompilationOptions;
use crate::records::compilation_result::CompilationResult;
use crate::records::compilation_stats::CompilationStats;
use crate::type_aliases::lua_state::lua_State;
use crate::type_aliases::module_id::ModuleId;

/// **Out of scope.** `Luau::CodeGen::compileInternal` generates and installs
/// native machine code; luaur's execution oracle is the bytecode interpreter
/// (see docs/CONFORMANCE.md), so this was never ported. It survived only as an
/// `extern` declaration of the original C++ mangled symbol, which has no
/// implementation to link against (lld DCE'd the reference on Linux/macOS, MSVC
/// kept it → unresolved external symbol on Windows). Stub it explicitly.
#[allow(unused_variables)]
pub fn compile_module_id_lua_state_i32_compilation_options_compilation_stats(
    module_id: &ModuleId,
    l: *mut lua_State,
    idx: i32,
    options: &CompilationOptions,
    stats: *mut CompilationStats,
) -> CompilationResult {
    unimplemented!(
        "luaur does not execute JIT-compiled native code (out of scope; see docs/CONFORMANCE.md)"
    )
}
