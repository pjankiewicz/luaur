use crate::functions::lua_l_argerror_l::lua_l_argerror_l;
use crate::functions::lua_l_checklstring::lua_l_checklstring;
use crate::functions::lua_l_optlstring::lua_l_optlstring;
use crate::functions::lua_pushfstring_l::lua_pushfstring_l;
use crate::type_aliases::lua_state::lua_State;
use core::ffi::c_char;
use core::ffi::c_int;
use core::ffi::CStr;

#[export_name = "luaur_luaL_checkoption"]
pub unsafe fn luaL_checkoption(
    L: *mut lua_State,
    narg: c_int,
    def: *const c_char,
    lst: *const *const c_char,
) -> c_int {
    let name: *const c_char = if !def.is_null() {
        lua_l_optlstring(L, narg, def, core::ptr::null_mut())
    } else {
        lua_l_checklstring(L, narg, core::ptr::null_mut())
    };

    let mut i: c_int = 0;
    while !(*lst.add(i as usize)).is_null() {
        let opt = *lst.add(i as usize);
        if libc_strcmp(opt, name) == 0 {
            return i;
        }
        i += 1;
    }

    let name_str = CStr::from_ptr(name).to_string_lossy();
    let msg = lua_pushfstring_l(
        L,
        c"invalid option '%s'".as_ptr(),
        format_args!("invalid option '{}'", name_str),
    );
    let msg_str = CStr::from_ptr(msg).to_string_lossy();
    lua_l_argerror_l(L, narg, msg_str.as_ref())
}

#[allow(non_snake_case)]
pub fn lua_l_checkoption(
    L: *mut lua_State,
    narg: c_int,
    def: Option<&str>,
    lst: *const *const c_char,
) -> c_int {
    unsafe {
        let def_cstring =
            def.map(|s| std::ffi::CString::new(s).expect("option default contains nul"));
        let def_ptr = match def_cstring.as_ref() {
            Some(s) => s.as_ptr(),
            None => core::ptr::null(),
        };
        luaL_checkoption(L, narg, def_ptr, lst)
    }
}

unsafe fn libc_strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
    let mut i = 0;
    loop {
        let c1 = *s1.add(i) as u8;
        let c2 = *s2.add(i) as u8;
        if c1 != c2 {
            return if c1 < c2 { -1 } else { 1 };
        }
        if c1 == 0 {
            return 0;
        }
        i += 1;
    }
}
