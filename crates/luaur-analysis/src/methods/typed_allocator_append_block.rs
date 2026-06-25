use crate::functions::paged_allocate::paged_allocate;
use crate::records::typed_allocator::TypedAllocator;

impl<T> TypedAllocator<T> {
    pub(crate) fn append_block(&mut self) {
        // Commit to an allocation strategy on the first block and keep it for the
        // allocator's whole lifetime, so `free` deallocates the way it allocated
        // even if the (ScopedFastFlag) DebugLuauFreezeArena flag is toggled in
        // between. Reading the flag per-call mismatched VirtualFree/operator-delete
        // and corrupted the heap on Windows.
        if self.stuff.is_empty() {
            self.paged = luaur_common::FFlag::DebugLuauFreezeArena.get();
        }
        let block = paged_allocate(Self::kBlockSizeBytes, self.paged);
        if block.is_null() {
            panic!("std::bad_alloc");
        }

        self.stuff.push(block as *mut T);
        self.current_block_size = 0;
    }
}
