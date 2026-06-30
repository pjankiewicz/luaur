use core::ffi::c_char;

use luaur_vm::macros::clvalue::clvalue;
use luaur_vm::macros::lua_g_typeerror::luaG_typeerror;
use luaur_vm::macros::ttisfunction::ttisfunction;
use luaur_vm::type_aliases::lua_state::lua_State;
use luaur_vm::type_aliases::t_value::TValue;

const ITERATE_OVER: *const c_char = b"iterate over\0".as_ptr() as *const c_char;

#[allow(non_snake_case)]
pub unsafe fn forg_prep_xnext_fallback(L: *mut lua_State, ra: *mut TValue, pc: i32) {
    if !ttisfunction!(ra as *const TValue) {
        let cl = clvalue!((*(*L).ci).func as *const TValue);
        let cl_l = &(*cl).inner.l;
        (*(*L).ci).savedpc = cl_l.p.as_ref().unwrap().code.add(pc as usize);

        luaG_typeerror!(L, ra as *const TValue, ITERATE_OVER);
    }
}

#[export_name = "luaur_forgPrepXnextFallback"]
pub unsafe extern "C" fn forgPrepXnextFallback(L: *mut lua_State, ra: *mut TValue, pc: i32) {
    forg_prep_xnext_fallback(L, ra, pc);
}
