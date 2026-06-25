#[cfg(not(target_os = "windows"))]
use core::ffi::{c_int, c_void};

#[cfg(not(any(target_os = "windows", target_os = "freebsd")))]
extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: isize,
    ) -> *mut c_void;
}

/// The OS page size.
///
/// Mirrors C++ `kPageSize`: `sysconf(_SC_PAGESIZE)` on POSIX, `getpagesize()`
/// on FreeBSD, 4096 on Win32.
#[cfg(not(target_os = "windows"))]
pub(crate) fn page_size() -> usize {
    #[cfg(target_os = "freebsd")]
    {
        extern "C" {
            fn getpagesize() -> c_int;
        }
        unsafe { getpagesize() as usize }
    }

    #[cfg(not(target_os = "freebsd"))]
    {
        extern "C" {
            fn sysconf(name: c_int) -> isize;
        }
        // `_SC_PAGESIZE` is NOT the same number on every platform: it is 30 on
        // Linux/Android but 29 on macOS/Darwin. The constant `29` here was the
        // Darwin value — on Linux 29 is `_SC_VERSION`, so `sysconf(29)` returned
        // the POSIX version (~200809), which is not a power of two and made every
        // arena `Layout::from_size_align` fail (→ paged_allocate returns null →
        // the allocator panics → the whole type checker falls over on Linux).
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const _SC_PAGESIZE: c_int = 30;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const _SC_PAGESIZE: c_int = 29;
        let v = unsafe { sysconf(_SC_PAGESIZE) };
        // Defensive: a page size must be a positive power of two (it is the
        // alignment we hand to `Layout`). If sysconf ever returns something
        // unexpected, fall back to 4 KiB rather than abort every allocation.
        if v >= 1 && (v as usize).is_power_of_two() {
            v as usize
        } else {
            4096
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn page_size() -> usize {
    4096
}

/// Round `size` up to a multiple of the OS page size.
///
/// Mirrors C++ `pageAlign`: `(size + kPageSize - 1) & ~(kPageSize - 1)`.
pub(crate) fn page_align(size: usize) -> usize {
    let page = page_size();
    (size + page - 1) & !(page - 1)
}

/// Port of `Luau::pagedAllocate`.
///
/// By default we use operator new/delete instead of malloc/free so that they
/// can be overridden externally. When `DebugLuauFreezeArena` is set, we use
/// page-granular OS allocation so the blocks can later be frozen with
/// `mprotect`/`VirtualProtect`.
pub fn paged_allocate(size: usize, freeze: bool) -> *mut core::ffi::c_void {
    // `freeze` is the allocation strategy chosen by the *caller* (the owning
    // TypedAllocator captures `DebugLuauFreezeArena` once, at its first
    // allocation, and threads the same value into both allocate and deallocate).
    // It must NOT be re-read from the global flag here: the flag is a toggleable
    // ScopedFastFlag in tests, so reading it at free time could pick a different
    // strategy than was used to allocate — e.g. VirtualFree on heap memory or
    // operator delete on VirtualAlloc memory — corrupting the heap. That mismatch
    // is what crashed ~all type-checking tests on Windows (0xC0000005 / VirtualFree
    // returning 0 at paged_deallocate). See `paged_deallocate`.
    if !freeze {
        // `::operator new(size, std::nothrow)` — a heap allocation that returns
        // null on failure. The matching `::operator delete` lives in
        // `paged_deallocate`; both reconstruct the identical `Layout` from the
        // size the caller passes (always `kBlockSizeBytes`).
        if size == 0 {
            return core::ptr::null_mut();
        }
        let layout = match core::alloc::Layout::from_size_align(size, page_size()) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        return unsafe { alloc::alloc::alloc(layout) as *mut core::ffi::c_void };
    }

    // On Windows, VirtualAlloc results in 64K granularity allocations; on
    // Linux we must use mmap because using the regular heap results in
    // mprotect() fragmenting the page table and bumping into the 64K mmap
    // limit.
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE,
        };
        unsafe {
            VirtualAlloc(
                core::ptr::null(),
                size,
                MEM_RESERVE | MEM_COMMIT,
                PAGE_READWRITE,
            ) as *mut core::ffi::c_void
        }
    }

    #[cfg(target_os = "freebsd")]
    {
        extern "C" {
            fn aligned_alloc(alignment: usize, size: usize) -> *mut c_void;
        }
        unsafe { aligned_alloc(page_size(), size) }
    }

    #[cfg(not(any(target_os = "windows", target_os = "freebsd")))]
    unsafe {
        const PROT_READ: c_int = 0x1;
        const PROT_WRITE: c_int = 0x2;
        const MAP_PRIVATE: c_int = 0x02;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const MAP_ANON: c_int = 0x20;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const MAP_ANON: c_int = 0x1000;

        let result = mmap(
            core::ptr::null_mut(),
            page_align(size),
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANON,
            -1,
            0,
        );

        // mmap returns MAP_FAILED (-1) on error. Normalize it to null so the
        // caller's null-check (`appendBlock`) observes the failure.
        if result as isize == -1 {
            core::ptr::null_mut()
        } else {
            result
        }
    }
}
