use crate::records::typed_allocator::TypedAllocator;

impl<T> TypedAllocator<T> {
    pub fn typed_allocator() -> Self {
        Self {
            frozen: false,
            stuff: alloc::vec::Vec::new(),
            current_block_size: Self::kBlockSize,
            paged: false,
        }
    }
}
