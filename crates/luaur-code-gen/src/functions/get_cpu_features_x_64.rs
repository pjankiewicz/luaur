pub fn get_cpu_features_x_64() -> u32 {
    let mut result: u32 = 0;

    let mut cpuinfo: [i32; 4] = [0, 0, 0, 0];

    if crate::macros::codegen_target_x_64::CODEGEN_TARGET_X64 {
        #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
        {
            // Use the `core::arch` CPUID intrinsic rather than hand-rolled inline
            // asm: the `cpuid` instruction clobbers `rbx`/`ebx`, which LLVM
            // reserves and forbids as an `asm!` operand ("rbx is used internally
            // by LLVM"). `__cpuid` saves/restores it for us and returns a
            // `CpuidResult { eax, ebx, ecx, edx }`. Works on every x86 OS.
            #[cfg(target_arch = "x86_64")]
            {
                unsafe {
                    let r = core::arch::x86_64::__cpuid(1);
                    cpuinfo = [r.eax as i32, r.ebx as i32, r.ecx as i32, r.edx as i32];
                }
            }

            #[cfg(target_arch = "x86")]
            {
                unsafe {
                    let r = core::arch::x86::__cpuid(1);
                    cpuinfo = [r.eax as i32, r.ebx as i32, r.ecx as i32, r.edx as i32];
                }
            }
        }
    }

    let feature_fma3 = crate::enums::features_x_64::FeaturesX64::Feature_FMA3 as u32;
    let feature_avx = crate::enums::features_x_64::FeaturesX64::Feature_AVX as u32;

    if (cpuinfo[2] & 0x00001000) != 0 {
        result |= feature_fma3;
    }

    if (cpuinfo[2] & 0x10000000) != 0 {
        result |= feature_avx;
    }

    result
}
