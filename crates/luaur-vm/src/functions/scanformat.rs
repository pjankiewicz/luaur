use crate::macros::lua_l_error::luaL_error;
use crate::macros::uchar::uchar;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_char;
use core::ptr;

#[export_name = "luaur_scanformat"]
pub unsafe fn scanformat(
    L: *mut lua_State,
    strfrmt: *const c_char,
    mut form: *mut c_char,
    size: *mut usize,
) -> *const c_char {
    const FLAGS: &[u8] = b"-+ #0";

    let mut p = strfrmt;
    while *p != 0 && FLAGS.contains(&(uchar(*p as i32) as u8)) {
        p = p.offset(1);
    }

    if (p as usize - strfrmt as usize) >= FLAGS.len() {
        luaL_error!(L, "invalid format (repeated flags)");
    }

    if (uchar(*p as i32) as u8).is_ascii_digit() {
        p = p.offset(1);
    }
    if (uchar(*p as i32) as u8).is_ascii_digit() {
        p = p.offset(1);
    }

    if *p == b'.' as c_char {
        p = p.offset(1);
        if (uchar(*p as i32) as u8).is_ascii_digit() {
            p = p.offset(1);
        }
        if (uchar(*p as i32) as u8).is_ascii_digit() {
            p = p.offset(1);
        }
    }

    if (uchar(*p as i32) as u8).is_ascii_digit() {
        luaL_error!(L, "invalid format (width or precision too long)");
    }

    ptr::write(form, b'%' as c_char);
    form = form.offset(1);
    *size = (p as usize - strfrmt as usize) + 1;
    core::ptr::copy_nonoverlapping(strfrmt, form, *size);
    form = form.offset(*size as isize);
    ptr::write(form, 0);

    p
}
