use crate::enums::lua_type::lua_Type;
use crate::macros::lua_c_init::luaC_init;
use crate::macros::setnilvalue::setnilvalue;
use crate::macros::size_lclosure::size_lclosure;
use crate::records::closure::{Closure, LClosure};
use crate::records::lua_table::LuaTable;
use crate::records::proto::Proto;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_int;

#[allow(non_snake_case)]
pub unsafe fn luaF_newLclosure(
    l: *mut lua_State,
    nelems: c_int,
    e: *mut LuaTable,
    p: *mut Proto,
) -> *mut Closure {
    let c = crate::functions::lua_m_newgco::luaM_newgco_(
        l,
        size_lclosure(nelems as usize),
        (*l).activememcat,
    ) as *mut Closure;

    luaC_init!(l, c, lua_Type::LUA_TFUNCTION as c_int);
    (*c).isC = 0;
    (*c).env = e;
    (*c).nupvalues = nelems as u8;
    (*c).stacksize = (*p).maxstacksize;
    (*c).preload = 0;
    (*c).usage = 0;
    (*c).gclist = core::ptr::null_mut();
    let lc = core::ptr::addr_of_mut!((*c).inner.l) as *mut LClosure;
    (*lc).p = p;

    let mut i = 0;
    while i < nelems {
        setnilvalue!((*lc).uprefs.as_mut_ptr().add(i as usize));
        i += 1;
    }

    c
}

#[allow(unused_imports)]
pub use luaF_newLclosure as lua_f_new_lclosure;

#[export_name = "luaur_luaF_newLclosure"]
pub unsafe extern "C" fn lua_f_new_lclosure_export(
    l: *mut lua_State,
    nelems: c_int,
    e: *mut core::ffi::c_void,
    p: *mut core::ffi::c_void,
) -> *mut core::ffi::c_void {
    luaF_newLclosure(l, nelems, e as *mut LuaTable, p as *mut Proto).cast()
}
