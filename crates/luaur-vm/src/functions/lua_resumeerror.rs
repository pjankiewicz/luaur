use crate::enums::lua_status::lua_Status;
use crate::functions::lua_d_rawrunprotected_ldo::lua_d_rawrunprotected;
use crate::functions::resume_findhandler::resume_findhandler;
use crate::functions::resume_finish::resume_finish;
use crate::functions::resume_handle::resume_handle;
use crate::functions::resume_start::resume_start;
use crate::macros::cast_byte::cast_byte;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_resumeerror"]
pub unsafe fn lua_resumeerror(L: *mut lua_State, from: *mut lua_State) -> i32 {
    let starterror = resume_start(L, from, 1);
    if starterror != 0 {
        return starterror;
    }

    let old_n_c_calls = (*L).nCcalls;
    let old_n_c_calls_i32: i32 = old_n_c_calls as i32;

    let status = lua_Status::LUA_ERRRUN as i32;

    let ci = resume_findhandler(L);
    if !ci.is_null() {
        (*L).status = cast_byte!(status);
        let status_result =
            lua_d_rawrunprotected(L, Some(resume_handle), ci as *mut core::ffi::c_void);
        return resume_finish(L, status_result, old_n_c_calls_i32);
    }

    resume_finish(L, status, old_n_c_calls_i32)
}
