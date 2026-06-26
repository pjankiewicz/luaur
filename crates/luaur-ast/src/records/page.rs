// C++ `alignas(8)` on `data`; Rust has no field-level alignment, but aligning
// the whole struct to 8 puts `data` (after the 8-byte `next` pointer) at an
// 8-aligned offset, preserving the intent.
#[repr(C, align(8))]
#[derive(Debug)]
#[allow(non_camel_case_types)]
pub struct Page {
    pub(crate) next: *mut Page,
    /// Total byte size of *this page's* heap allocation (`offset_of!(Page, data) +
    /// page_size`). C++ frees pages with `operator delete(page)`, which recovers
    /// the size from the allocator; Rust's `dealloc` requires the exact `Layout`,
    /// and a page can be over-sized for a single large allocation, so we record
    /// the real size here to free each page correctly. Not in the C++ struct — a
    /// Rust-allocator necessity, not a semantic change.
    pub(crate) alloc_size: usize,
    pub(crate) data: [u8; 8192],
}
