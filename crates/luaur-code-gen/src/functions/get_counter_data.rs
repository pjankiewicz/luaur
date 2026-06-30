use crate::functions::get_native_proto_exec_data_header_native_proto_exec_data_alt_b::get_native_proto_exec_data_header;
use crate::macros::codegen_assert::CODEGEN_ASSERT;

pub fn get_counter_data(
    l: *mut crate::type_aliases::lua_state::lua_State,
    proto: *mut luaur_vm::records::proto::Proto,
    count: *mut usize,
) -> *mut core::ffi::c_char {
    let _ = l;

    unsafe {
        CODEGEN_ASSERT!(!count.is_null());

        let exec_data = (*proto).execdata as *mut u32;
        let exec_data_header = &*get_native_proto_exec_data_header(exec_data);

        *count = exec_data_header.extra_data_count as usize / 4;
        exec_data.add((*proto).sizecode as usize) as *mut core::ffi::c_char
    }
}

#[export_name = "luaur_get_counter_data"]
pub unsafe extern "C" fn get_counter_data_export(
    l: *mut crate::type_aliases::lua_state::lua_State,
    proto: *mut luaur_vm::records::proto::Proto,
    count: *mut usize,
) -> *mut core::ffi::c_char {
    get_counter_data(l, proto, count)
}
