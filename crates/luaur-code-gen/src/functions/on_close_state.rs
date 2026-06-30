use crate::type_aliases::lua_state::lua_State;
use luaur_vm::records::lua_execution_callbacks::lua_ExecutionCallbacks;

pub fn on_close_state(L: *mut lua_State) {
    unsafe {
        if L.is_null() {
            return;
        }

        let l_internal = L as *mut luaur_vm::records::lua_state::lua_State;
        let global = (*l_internal).global;
        if !global.is_null() {
            (*global).ecb = lua_ExecutionCallbacks {
                context: core::ptr::null_mut(),
                close: None,
                destroy: None,
                enter: None,
                disable: None,
                getmemorysize: None,
                gettypemapping: None,
                getcounterdata: None,
                inlinefunction: None,
            };
        }
    }
}

#[export_name = "luaur_on_close_state"]
pub unsafe extern "C" fn on_close_state_export(L: *mut lua_State) {
    on_close_state(L);
}
