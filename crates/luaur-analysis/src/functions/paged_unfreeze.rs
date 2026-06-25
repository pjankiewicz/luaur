use luaur_common::macros::luau_assert::LUAU_ASSERT;
use luaur_common::FFlag;

use crate::functions::paged_allocate::{page_align, page_size};

#[cfg(not(target_os = "windows"))]
use core::ffi::{c_int, c_void};

#[cfg(not(target_os = "windows"))]
extern "C" {
    fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
}

/// Port of `Luau::pagedUnfreeze`.
///
/// Restores read-write access to `[ptr, ptr + pageAlign(size))` previously
/// frozen by `paged_freeze`. Only valid when `DebugLuauFreezeArena` is set and
/// `ptr` is page aligned.
pub fn paged_unfreeze(ptr: *mut core::ffi::c_void, size: usize) {
    LUAU_ASSERT!(FFlag::DebugLuauFreezeArena.get());
    LUAU_ASSERT!((ptr as usize) % page_size() == 0);

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::{VirtualProtect, PAGE_READWRITE};
        let mut old_protect: u32 = 0;
        let rc =
            unsafe { VirtualProtect(ptr, page_align(size), PAGE_READWRITE, &mut old_protect) };
        LUAU_ASSERT!(rc != 0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        const PROT_READ: c_int = 0x1;
        const PROT_WRITE: c_int = 0x2;
        let rc = unsafe { mprotect(ptr as *mut c_void, page_align(size), PROT_READ | PROT_WRITE) };
        LUAU_ASSERT!(rc == 0);
    }
}
