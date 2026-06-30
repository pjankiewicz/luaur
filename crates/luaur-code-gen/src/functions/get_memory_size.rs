use crate::functions::get_native_proto_exec_data_header_native_proto_exec_data::get_native_proto_exec_data_header_mut;
use crate::records::native_proto_exec_data_header::NativeProtoExecDataHeader;
use crate::type_aliases::instruction_ir_builder::Instruction;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::records::proto::Proto;

pub fn get_memory_size(L: *mut lua_State, proto: *mut Proto) -> usize {
    let proto_ref = unsafe { &*proto };
    let exec_data_header =
        unsafe { &*get_native_proto_exec_data_header_mut(proto_ref.execdata as *mut u32) };

    let exec_data_size = core::mem::size_of::<NativeProtoExecDataHeader>()
        + (exec_data_header.bytecode_instruction_count as usize)
            * core::mem::size_of::<Instruction>();

    exec_data_size + exec_data_header.native_code_size
}

#[export_name = "luaur_get_memory_size"]
pub unsafe extern "C" fn get_memory_size_export(L: *mut lua_State, proto: *mut Proto) -> usize {
    get_memory_size(L, proto)
}
