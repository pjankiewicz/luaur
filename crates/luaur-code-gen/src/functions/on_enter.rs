use crate::functions::get_code_gen_context::get_code_gen_context;
use crate::macros::codegen_assert::CODEGEN_ASSERT;
use crate::records::native_context::NativeContext;

pub fn on_enter(
    l: *mut crate::type_aliases::lua_state::lua_State,
    proto: *mut luaur_vm::records::proto::Proto,
) -> i32 {
    type GateFn = unsafe extern "C" fn(
        *mut crate::type_aliases::lua_state::lua_State,
        *mut luaur_vm::records::proto::Proto,
        usize,
        *mut NativeContext,
    ) -> core::ffi::c_int;

    unsafe {
        let code_gen_context = get_code_gen_context(l);

        CODEGEN_ASSERT!(!code_gen_context.is_null());
        CODEGEN_ASSERT!(!(*proto).execdata.is_null());
        CODEGEN_ASSERT!((*(*l).ci).savedpc >= (*proto).code);
        CODEGEN_ASSERT!((*(*l).ci).savedpc < (*proto).code.add((*proto).sizecode as usize));

        let pc_offset = (*(*l).ci).savedpc.offset_from((*proto).code) as usize;
        let instruction_offsets = (*proto).execdata as *mut u32;
        let target = (*proto).exectarget + *instruction_offsets.add(pc_offset) as usize;

        let gate: GateFn = core::mem::transmute((*code_gen_context).context.gateEntry);
        gate(l, proto, target, &mut (*code_gen_context).context)
    }
}

#[export_name = "luaur_on_enter"]
pub unsafe extern "C" fn on_enter_export(
    l: *mut crate::type_aliases::lua_state::lua_State,
    proto: *mut luaur_vm::records::proto::Proto,
) -> core::ffi::c_int {
    on_enter(l, proto)
}
