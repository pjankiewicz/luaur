use crate::macros::codegen_assert::CODEGEN_ASSERT;
use crate::records::code_allocator::CodeAllocator;
use core::ffi::c_void;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READ};

#[allow(non_snake_case)]
pub fn make_pages_executable(mem: *mut u8, size: usize) -> bool {
    CODEGEN_ASSERT!(CodeAllocator::align_to_page_size(mem as usize) == mem as usize);
    CODEGEN_ASSERT!(size == CodeAllocator::align_to_page_size(size));

    #[cfg(target_os = "windows")]
    {
        let mut old_protect: u32 = 0;
        unsafe {
            VirtualProtect(
                mem as *const c_void,
                size,
                PAGE_EXECUTE_READ,
                &mut old_protect,
            ) != 0
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        unsafe {
            crate::functions::make_pages_executable_code_allocator_alt_b::make_pages_executable_mut(
                mem, size,
            )
        }
    }
}
