use crate::functions::get_code_gen_context::get_code_gen_context;
use crate::records::shared_code_gen_context::SharedCodeGenContext;
use crate::type_aliases::lua_state::lua_State;
use luaur_vm::records::proto::Proto;

pub fn on_destroy_function(L: *mut lua_State, proto: *mut Proto) {
    unsafe {
        if L.is_null() || proto.is_null() {
            return;
        }

        let ctx = get_code_gen_context(L);
        if !ctx.is_null() && !(*proto).execdata.is_null() {
            SharedCodeGenContext::on_destroy_function((*proto).execdata);
        }

        (*proto).execdata = core::ptr::null_mut();
        (*proto).exectarget = 0;
        (*proto).codeentry = (*proto).code;
    }
}

#[export_name = "luaur_on_destroy_function"]
pub unsafe extern "C" fn on_destroy_function_export(L: *mut lua_State, proto: *mut Proto) {
    on_destroy_function(L, proto);
}
