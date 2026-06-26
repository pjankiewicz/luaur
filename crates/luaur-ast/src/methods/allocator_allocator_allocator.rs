use crate::records::allocator::Allocator;
use crate::records::page::Page;
use alloc::alloc::{alloc, Layout};

#[allow(non_snake_case)]
impl Allocator {
    pub fn allocator() -> Self {
        unsafe {
            let layout = Layout::new::<Page>();
            let ptr = alloc(layout) as *mut Page;
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            (*ptr).next = core::ptr::null_mut();
            // Record the allocation size so `Drop` frees this initial page with
            // the matching `Layout` (see `Page::alloc_size`).
            (*ptr).alloc_size = layout.size();
            Allocator {
                root: ptr,
                offset: 0,
            }
        }
    }
}
