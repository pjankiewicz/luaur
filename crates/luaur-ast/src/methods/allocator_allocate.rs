use crate::records::allocator::Allocator;
use crate::records::page::Page;
use alloc::alloc::{alloc, Layout};

#[allow(non_snake_case)]
impl Allocator {
    pub fn allocate(&mut self, size: usize) -> *mut u8 {
        let align_void = core::mem::align_of::<*mut ()>();
        let align_double = core::mem::align_of::<f64>();
        let align = if align_void > align_double {
            align_void
        } else {
            align_double
        };

        unsafe {
            if !self.root.is_null() {
                let data_ptr = (*self.root).data.as_ptr() as usize;
                let result = (data_ptr + self.offset + align - 1) & !(align - 1);

                if result + size <= data_ptr + (*self.root).data.len() {
                    self.offset = result - data_ptr + size;
                    return result as *mut u8;
                }
            }

            // allocate new page
            let default_page_data_size = core::mem::size_of::<[u8; 8192]>();
            let page_data_offset = core::mem::offset_of!(Page, data);
            let page_size = if size > default_page_data_size {
                size
            } else {
                default_page_data_size
            };

            let layout = Layout::from_size_align(
                page_data_offset + page_size,
                core::mem::align_of::<Page>(),
            )
            .expect("Invalid layout for Page allocation");

            let page_ptr = alloc(layout) as *mut Page;
            if page_ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }

            (*page_ptr).next = self.root;
            // Record the exact allocation size so `Drop` can free this page with
            // the matching `Layout` (pages may be over-sized for a large request).
            (*page_ptr).alloc_size = layout.size();
            self.root = page_ptr;
            self.offset = size;

            (*page_ptr).data.as_mut_ptr()
        }
    }
}
