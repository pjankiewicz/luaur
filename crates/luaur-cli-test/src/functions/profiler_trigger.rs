use core::ffi::{c_char, c_int, c_void};
use luaur_vm::functions::lua_getinfo::lua_getinfo;

// EXTERNAL_CRATE_REQUIRED: alloc - for String
use alloc::string::String;

// Global profiler state is defined in the C++ CLI profiler implementation and is not part of
// the provided translated Rust context for this one-shot item.
// Implementations of the referenced global state (gProfiler, its fields ticks, currentTicks,
// stackScratch, data, gc, and callbacks) must be provided by other translated items before this
// can be made functional.
extern "C" {
    #[link_name = "luaur_mut"]
    static mut gProfiler: c_void;
}

pub fn profiler_trigger(l: *mut c_void, gc: c_int) {
    // This function is native-only and relies on global state (gProfiler) and
    // lua_getinfo which is already translated but requires a lua_State* context.
    //
    // The full implementation is not possible without the global profiler state
    // and its fields being available in the translated Rust context.
    //
    // The following is a stub that matches the signature and documents the
    // required context for future implementation.

    let _ = (l, gc);
    let _ = lua_getinfo;
}
