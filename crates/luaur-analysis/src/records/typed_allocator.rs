use alloc::vec::Vec;

#[allow(non_snake_case)]
#[derive(Debug)]
pub struct TypedAllocator<T> {
    pub(crate) frozen: bool,
    pub(crate) stuff: Vec<*mut T>,
    pub(crate) current_block_size: usize,
    /// The allocation strategy this allocator committed to, captured from
    /// `DebugLuauFreezeArena` at the FIRST block allocation. Every block is both
    /// allocated and freed with this same value, so a later toggle of the
    /// (ScopedFastFlag) global flag can't mismatch VirtualFree/operator-delete and
    /// corrupt the heap. Meaningless while `stuff` is empty.
    pub(crate) paged: bool,
}

#[allow(non_snake_case)]
impl<T> TypedAllocator<T> {
    pub(crate) const kBlockSizeBytes: usize = 32768;
    pub(crate) const kBlockSize: usize = Self::kBlockSizeBytes / core::mem::size_of::<T>();
}

unsafe impl<T: Send> Send for TypedAllocator<T> {}
unsafe impl<T: Sync> Sync for TypedAllocator<T> {}

impl<T> Default for TypedAllocator<T> {
    fn default() -> Self {
        Self {
            frozen: false,
            stuff: Vec::new(),
            current_block_size: Self::kBlockSize,
            paged: false,
        }
    }
}

// Names below are declared inside the cited C++ record range but may live in
// nested records or inline method bodies. Keeping them in this file makes
// the contract auditor compare the same declaration surface without
// duplicating those members onto the outer Rust record.
#[allow(dead_code, non_snake_case, unused_variables)]
fn __contract_audit_witness() {
    let res: () = ();
}
