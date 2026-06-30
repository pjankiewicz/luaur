use crate::functions::auxresume::auxresume;
use crate::functions::auxwrapfinish::auxwrapfinish;
use crate::functions::interrupt_thread::interrupt_thread;
use crate::functions::lua_tothread::lua_tothread;
use crate::macros::cast_int::cast_int;
use crate::macros::co_status_break::CO_STATUS_BREAK;
use crate::macros::lua_upvalueindex::lua_upvalueindex;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_auxwrapy"]
pub unsafe fn auxwrapy(L: *mut lua_State) -> core::ffi::c_int {
    let co = lua_tothread(L, lua_upvalueindex(1));
    let narg = cast_int!((*L).top.offset_from((*L).base));
    let r = auxresume(L, co, narg);
    if r == CO_STATUS_BREAK {
        interrupt_thread(L, co)
    } else {
        auxwrapfinish(L, r)
    }
}
