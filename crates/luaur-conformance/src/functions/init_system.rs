use luaur_code_gen::macros::codegen_target_x_64::CODEGEN_TARGET_X64;

pub fn init_system() {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        if CODEGEN_TARGET_X64 {
            unsafe {
                use core::arch::x86_64::*;

                // Some unit tests make use of denormalized numbers. So flags to flush to zero or treat denormals as zero
                // must be disabled for expected behavior.
                _MM_SET_FLUSH_ZERO_MODE(_MM_FLUSH_ZERO_OFF);
                // `_MM_SET_DENORMALS_ZERO_MODE` / `_MM_DENORMALS_ZERO_OFF` are not
                // exposed by name in Rust's core::arch, so clear the DAZ bit
                // (denormals-are-zero, MXCSR bit 6 = 0x0040) directly via MXCSR.
                #[allow(deprecated)]
                {
                    _mm_setcsr(_mm_getcsr() & !0x0040);
                }
            }
        }
    }
}
