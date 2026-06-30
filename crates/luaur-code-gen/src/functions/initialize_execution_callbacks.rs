use crate::macros::codegen_assert::CODEGEN_ASSERT;
use crate::records::base_code_gen_context::BaseCodeGenContext;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::records::lua_execution_callbacks::lua_ExecutionCallbacks;

pub fn initialize_execution_callbacks(
    L: *mut lua_State,
    code_gen_context: *mut BaseCodeGenContext,
) {
    CODEGEN_ASSERT!(!code_gen_context.is_null());

    unsafe {
        let ecb: *mut lua_ExecutionCallbacks = &mut (*(*L).global).ecb;

        (*ecb).context = code_gen_context as *mut core::ffi::c_void;
        (*ecb).close = Some(on_close_state);
        (*ecb).destroy = Some(on_destroy_function);
        (*ecb).enter = Some(on_enter);
        (*ecb).disable = Some(on_disable);
        (*ecb).getmemorysize = Some(get_memory_size);
        (*ecb).getcounterdata = Some(get_counter_data);
    }
}

extern "C" {
    #[link_name = "luaur_on_close_state"]
    fn on_close_state(L: *mut lua_State);
    #[link_name = "luaur_on_destroy_function"]
    fn on_destroy_function(L: *mut lua_State, proto: *mut luaur_vm::records::proto::Proto);
    #[link_name = "luaur_on_enter"]
    fn on_enter(L: *mut lua_State, proto: *mut luaur_vm::records::proto::Proto)
        -> core::ffi::c_int;
    #[link_name = "luaur_on_disable"]
    fn on_disable(L: *mut lua_State, proto: *mut luaur_vm::records::proto::Proto);
    #[link_name = "luaur_get_memory_size"]
    fn get_memory_size(L: *mut lua_State, proto: *mut luaur_vm::records::proto::Proto) -> usize;
    #[link_name = "luaur_get_counter_data"]
    fn get_counter_data(
        L: *mut lua_State,
        proto: *mut luaur_vm::records::proto::Proto,
        count: *mut usize,
    ) -> *mut core::ffi::c_char;
}
