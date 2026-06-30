use core::ffi::c_int;

use crate::enums::lua_type::lua_Type;
use crate::macros::api_check::api_check;
use crate::macros::setnilvalue::setnilvalue;
use crate::type_aliases::lua_state::lua_State;

#[export_name = "luaur_lua_settop"]
pub unsafe fn lua_settop(L: *mut lua_State, idx: c_int) {
    if idx >= 0 {
        api_check!(
            L,
            idx as isize <= unsafe { (*L).stack_last.offset_from((*L).base) }
        );
        while unsafe { (*L).top < (*L).base.add(idx as usize) } {
            setnilvalue!(unsafe { (*L).top });
            unsafe {
                (*L).top = (*L).top.add(1);
            }
        }
        unsafe {
            (*L).top = (*L).base.add(idx as usize);
        }
    } else {
        api_check!(
            L,
            -(idx + 1) as isize <= unsafe { (*L).top.offset_from((*L).base) }
        );
        unsafe {
            (*L).top = (*L).top.offset((idx + 1) as isize); // `subtract' index (index is negative)
        }
    }
}
