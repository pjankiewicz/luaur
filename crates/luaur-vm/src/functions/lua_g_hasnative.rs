use crate::functions::getluaproto::get_lua_proto;
use crate::records::call_info::CallInfo;
use crate::records::lua_state::lua_State;
use crate::type_aliases::lua_state::lua_State as LuaState;
use crate::type_aliases::proto::Proto;

#[export_name = "luaur_lua_g_hasnative"]
pub unsafe fn lua_g_hasnative(L: *mut lua_State, level: core::ffi::c_int) -> core::ffi::c_int {
    if (level as u32) >= ((*L).ci).offset_from((*L).base_ci) as u32 {
        return 0;
    }

    let ci: *mut CallInfo = (*L).ci.offset(-(level as isize));
    let proto: *mut Proto = get_lua_proto(ci);
    if proto.is_null() {
        return 0;
    }

    ((*proto).execdata).is_null() as core::ffi::c_int ^ 1
}
