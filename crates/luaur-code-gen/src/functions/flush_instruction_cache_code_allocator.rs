use crate::functions::flush_instruction_cache_code_allocator_alt_b::flush_instruction_cache_mut;
use crate::macros::codegen_assert::CODEGEN_ASSERT;

#[allow(non_snake_case)]
pub fn flush_instruction_cache(mem: *mut u8, size: usize) {
    #[cfg(target_os = "windows")]
    {
        use core::ffi::c_void;
        use windows_sys::Win32::System::Diagnostics::Debug::FlushInstructionCache;
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        unsafe {
            if FlushInstructionCache(GetCurrentProcess(), mem as *const c_void, size) == 0 {
                CODEGEN_ASSERT!(false);
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        flush_instruction_cache_mut(mem, size);
    }
}
