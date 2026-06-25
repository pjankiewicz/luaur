use luaur_code_gen::macros::codegen_target_x_64::CODEGEN_TARGET_X64;

#[cfg(target_arch = "x86")]
use core::arch::x86 as x86_intrinsic;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86_intrinsic;

pub fn init_system() {
    if CODEGEN_TARGET_X64 {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            x86_intrinsic::_MM_SET_FLUSH_ZERO_MODE(x86_intrinsic::_MM_FLUSH_ZERO_OFF);
            // `_MM_SET_DENORMALS_ZERO_MODE` / `_MM_DENORMALS_ZERO_OFF` are not
            // exposed by name in Rust's core::arch, so clear the DAZ bit
            // (denormals-are-zero, MXCSR bit 6 = 0x0040) directly via MXCSR.
            #[allow(deprecated)]
            {
                x86_intrinsic::_mm_setcsr(x86_intrinsic::_mm_getcsr() & !0x0040);
            }
        }
    }
}
