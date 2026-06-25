use crate::functions::paged_deallocate::paged_deallocate;
use crate::records::typed_allocator::TypedAllocator;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

impl<T> TypedAllocator<T> {
    pub(crate) fn free(&mut self) {
        LUAU_ASSERT!(!self.frozen);

        let last_block = self.stuff.last().copied();

        for &block in &self.stuff {
            let block_size = if Some(block) == last_block {
                self.current_block_size
            } else {
                Self::kBlockSize
            };

            for i in 0..block_size {
                unsafe {
                    core::ptr::drop_in_place(block.add(i));
                }
            }

            paged_deallocate(block as *mut core::ffi::c_void, Self::kBlockSizeBytes, self.paged);
        }

        self.stuff.clear();
        self.current_block_size = 0;
    }
}
