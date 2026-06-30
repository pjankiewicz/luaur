use crate::functions::compute_native_exec_data_size::compute_native_exec_data_size;
use crate::functions::get_native_proto_exec_data_header_native_proto_exec_data_alt_b::get_native_proto_exec_data_header;
use crate::records::native_proto_exec_data_header::NativeProtoExecDataHeader;

/// # Safety
///
/// This function performs a manual memory deallocation of the `NativeProtoExecData`
/// structure. The caller must ensure that `instruction_offsets` points to a valid
/// `NativeProtoExecData` block previously allocated by `createNativeProtoExecData`.
/// The pointer must not be used after this call.
#[export_name = "luaur_destroy_native_proto_exec_data"]
pub unsafe extern "C" fn destroy_native_proto_exec_data(instruction_offsets: *const u32) {
    if instruction_offsets.is_null() {
        return;
    }

    let header = get_native_proto_exec_data_header(instruction_offsets);
    if header.is_null() {
        return;
    }

    // The C++ code calls the destructor and then deletes the memory block.
    // In Rust, we drop the header and then free the memory.
    // Since the header is at the start of the allocation, we cast the header
    // pointer to a byte pointer to free the entire block.
    unsafe {
        let bytecode_instruction_count = (*header).bytecode_instruction_count;
        let extra_data_count = (*header).extra_data_count;
        let layout = core::alloc::Layout::from_size_align(
            compute_native_exec_data_size(bytecode_instruction_count, extra_data_count),
            core::mem::align_of::<u32>(),
        )
        .expect("invalid NativeProtoExecData layout");
        core::ptr::drop_in_place(header as *mut NativeProtoExecDataHeader);
        alloc::alloc::dealloc(header as *mut u8, layout);
    }
}
