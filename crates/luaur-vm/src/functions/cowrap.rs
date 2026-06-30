use crate::functions::cocreate::cocreate;
use crate::functions::lua_pushcclosurek::lua_pushcclosurek;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_cowrap"]
pub unsafe fn cowrap(l: *mut lua_State) -> core::ffi::c_int {
    cocreate(l);
    lua_pushcclosurek(
        l,
        Some(crate::functions::auxwrapy::auxwrapy),
        core::ptr::null(),
        1,
        Some(crate::functions::auxwrapcont::auxwrapcont),
    );
    1
}
