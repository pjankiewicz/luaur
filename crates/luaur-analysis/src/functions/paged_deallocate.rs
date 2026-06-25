use luaur_common::macros::luau_assert::LUAU_ASSERT;

use crate::functions::paged_allocate::page_size;

#[cfg(not(target_os = "windows"))]
use core::ffi::{c_int, c_void};

#[cfg(not(any(target_os = "windows", target_os = "freebsd")))]
extern "C" {
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

/// Port of `Luau::pagedDeallocate`.
///
/// Frees a block previously returned by `paged_allocate`. `size` is always the
/// block size the matching allocation was made with (`kBlockSizeBytes`), which
/// lets the default heap path reconstruct the same `Layout`.
pub fn paged_deallocate(ptr: *mut core::ffi::c_void, size: usize, freeze: bool) {
    // `freeze` is the strategy the matching `paged_allocate` used (captured once by
    // the owning TypedAllocator). It must match the allocation — re-reading the
    // global DebugLuauFreezeArena flag here was the bug: it is a toggleable
    // ScopedFastFlag, so a block allocated under one value could be freed under the
    // other, mismatching VirtualFree/operator-delete and corrupting the heap (the
    // Windows 0xC0000005 / VirtualFree==0 failures).
    if !freeze {
        // `::operator delete(ptr)`. Reconstruct the exact `Layout` used by
        // `paged_allocate`'s default branch.
        if ptr.is_null() || size == 0 {
            return;
        }
        if let Ok(layout) = core::alloc::Layout::from_size_align(size, page_size()) {
            unsafe { alloc::alloc::dealloc(ptr as *mut u8, layout) };
        }
        return;
    }

    #[cfg(target_os = "windows")]
    {
        // The matching `paged_allocate` freeze-path used `VirtualAlloc`, so this
        // block is OS virtual memory and MUST be released with `VirtualFree(...,
        // MEM_RELEASE)`. The previous `_aligned_free` (a CRT-heap function) was
        // called on `VirtualAlloc`'d memory — a catastrophic allocator mismatch
        // that corrupted the heap and crashed ~every type-checking test on Windows
        // with 0xC0000005 (the test fixtures set DebugLuauFreezeArena=true, so all
        // their type arenas take this path). Linux/macOS were correct (mmap/munmap),
        // which is why the bug was Windows-only and invisible to valgrind on Linux.
        // MEM_RELEASE requires the size argument to be 0.
        use windows_sys::Win32::System::Memory::{VirtualFree, MEM_RELEASE};
        let _ = size;
        let rc = unsafe { VirtualFree(ptr, 0, MEM_RELEASE) };
        LUAU_ASSERT!(rc != 0);
    }

    #[cfg(target_os = "freebsd")]
    {
        extern "C" {
            fn free(ptr: *mut c_void);
        }
        unsafe { free(ptr) };
    }

    #[cfg(not(any(target_os = "windows", target_os = "freebsd")))]
    {
        let rc = unsafe { munmap(ptr as *mut c_void, size) };
        LUAU_ASSERT!(rc == 0);
    }
}
