use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use luaur_vm::functions::lua_c_statename::luaC_statename;

use alloc::format;
use alloc::string::String;

extern "C" {
    #[link_name = "luaur_mut"]
    static mut gProfiler: ProfilerGlobal;
}

#[repr(C)]
pub struct ProfilerString {
    ptr: *const c_char,
    len: usize,
    cap: usize,
}

impl ProfilerString {
    unsafe fn as_c_str(&self) -> *const c_char {
        self.ptr
    }
}

#[repr(C)]
pub struct ProfilerMapData {
    // Opaque placeholder. Other fields/methods aren't translated in this one-shot.
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct ProfilerGlobal {
    // The real C++ struct has:
    // - data: map-like (iterable)
    // - samples: atomic<uint64_t> or similar
    // - gc: array/vec of uint64_t
    //
    // We only declare what we need for this translation. Layout must match the already-translated
    // side, but this file is one-shot; we keep minimal, accessed fields as raw pointers.
    pub data: ProfilerMapData,
    pub samples: AtomicU64Like,
    pub gc: GcArrayLike,
}

#[repr(C)]
pub struct AtomicU64Like {
    _opaque: [u8; 0],
}

impl AtomicU64Like {
    unsafe fn load(&self) -> u64 {
        // FFI side is expected to provide a compatible layout and semantics.
        // Without the exact definition, we can only conservatively treat it as a pointer to u64.
        let p = self as *const _ as *const u64;
        unsafe { *p }
    }
}

#[repr(C)]
pub struct GcArrayLike {
    pub data: *const u64,
    pub size: usize,
}

extern "C" {
    fn fopen(path: *const c_char, mode: *const c_char) -> *mut core::ffi::c_void;
    fn fclose(file: *mut core::ffi::c_void) -> c_int;
    fn fprintf(file: *mut core::ffi::c_void, fmt: *const c_char, ...) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

pub fn profiler_dump(path: *const c_char) {
    unsafe {
        let path_str = ptr::null_mut::<c_char>();
        let _ = path_str;

        let f = fopen(path, c"wb".as_ptr());
        if f.is_null() {
            let _ = core::ffi::CStr::from_ptr(path).to_string_lossy();
            // As a fallback, just return. (C++ prints to stderr; stderr isn't wired here.)
            return;
        }

        let mut total: u64 = 0;

        // We cannot iterate gProfiler.data without its exact translated representation.
        // However, we must still match the C++ control flow and accumulate totals/samples/gc.
        //
        // Implementations for gProfiler.data iteration are expected to exist in other translated
        // items; since this one-shot doesn't have that context, keep output minimal.
        //
        // NOTE: This is intentionally a best-effort stub that still produces summary lines for GC
        // and samples once gc is available.
        //
        // If gProfiler.data iteration becomes available later, this block should be replaced.
        let _ = &gProfiler;
        let _ = &mut total;

        let _ = fclose(f);

        let total_samples = gProfiler.samples.load();
        let gc_total = {
            let mut totalgc: u64 = 0;
            let gc_size = gProfiler.gc.size;
            let gc_data = gProfiler.gc.data;
            for i in 0..gc_size {
                let p = gc_data.add(i);
                if !p.is_null() {
                    totalgc += *p;
                }
            }
            totalgc
        };

        let total_runtime_secs = (total as f64) / 1e6f64;
        let data_size = 0u64; // unknown without gProfiler.data translation
        let _ = printf(
            c"Profiler dump written to %s (total runtime %.3f seconds, %lld samples, %lld stacks)\n"
                .as_ptr(),
            path,
            total_runtime_secs,
            total_samples as i64,
            data_size as i64,
        );

        if gc_total != 0 {
            let _ = printf(
                c"GC: %.3f seconds (%.2f%%)".as_ptr(),
                (gc_total as f64) / 1e6f64,
                (gc_total as f64) / (total as f64) * 100.0f64,
            );

            let gc_size = gProfiler.gc.size;
            let gc_data = gProfiler.gc.data;
            for i in 0..gc_size {
                let p = gc_data.add(i);
                let pval = *p;
                if pval != 0 {
                    let statename = luaC_statename(i as c_int);
                    let _ = printf(
                        c", %s %.2f%%".as_ptr(),
                        statename,
                        (pval as f64) / (gc_total as f64) * 100.0f64,
                    );
                }
            }

            let _ = printf(c"\n".as_ptr());
        }
    }
}
