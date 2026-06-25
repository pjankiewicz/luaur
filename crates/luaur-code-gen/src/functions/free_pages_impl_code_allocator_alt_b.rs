use crate::macros::codegen_assert::CODEGEN_ASSERT;
use crate::records::code_allocator::CodeAllocator;
use core::ffi::c_void;

#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};

#[allow(non_snake_case)]
pub fn free_pages_impl_mut(mem: *mut u8, size: usize) {
    CODEGEN_ASSERT!(size == CodeAllocator::align_to_page_size(size));

    #[cfg(target_os = "windows")]
    {
        unsafe {
            if VirtualFree(mem as *mut c_void, 0, MEM_RELEASE) == 0 {
                CODEGEN_ASSERT!(false);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        use core::ffi::{c_int, c_void};

        extern "C" {
            fn munmap(addr: *mut c_void, len: usize) -> c_int;
        }

        unsafe {
            if munmap(mem as *mut c_void, size) != 0 {
                CODEGEN_ASSERT!(false);
            }
        }
    }
}
