use crate::records::shared_code_gen_context::SharedCodeGenContext;

/// # Safety
///
/// This function is native-only and performs a manual memory deallocation.
/// The caller must ensure that `code_gen_context` was created by a matching
/// allocation function and that it is not used after this call.
#[export_name = "luaur_destroy_shared_code_gen_context"]
pub unsafe extern "C" fn destroy_shared_code_gen_context(
    code_gen_context: *const SharedCodeGenContext,
) {
    if !code_gen_context.is_null() {
        let ptr = code_gen_context as *mut SharedCodeGenContext;
        unsafe {
            core::ptr::drop_in_place(ptr);
            alloc::alloc::dealloc(
                ptr as *mut u8,
                core::alloc::Layout::new::<SharedCodeGenContext>(),
            );
        }
    }
}
