use crate::type_aliases::allocation_callback::AllocationCallback;
use crate::type_aliases::lua_state::lua_State;

/// Create a standalone native-codegen context and wire its execution callbacks.
///
/// **Out of scope.** Native code *execution* is not part of luaur's validated
/// surface — the bytecode interpreter is the execution oracle (see
/// docs/CONFORMANCE.md). The C++ setup (`StandaloneCodeGenContext`,
/// `BaseCodeGenContext::initHeaderFunctions`, `initializeExecutionCallbacks`) was
/// never ported to Rust; it survived only as `extern` declarations of the
/// original C++ mangled symbols, which have no implementation to link against.
/// lld dead-code-eliminated those phantom references on Linux/macOS, but the MSVC
/// linker kept them and failed ("unresolved external symbol"). Replacing the
/// phantom externs with an explicit stub keeps the symbol set honest and lets the
/// workspace link on every platform.
#[allow(unused_variables)]
pub fn create_lua_state_usize_usize_allocation_callback_void(
    l: *mut lua_State,
    block_size: usize,
    max_total_size: usize,
    allocation_callback: *mut AllocationCallback,
    allocation_callback_context: *mut core::ffi::c_void,
) {
    unimplemented!("luaur does not execute JIT-compiled native code (out of scope; see docs/CONFORMANCE.md)")
}
