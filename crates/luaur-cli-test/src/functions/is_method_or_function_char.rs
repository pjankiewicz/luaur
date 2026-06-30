use core::ffi::{c_char, c_long};

#[export_name = "luaur_is_method_or_function_char"]
pub unsafe extern "C" fn is_method_or_function_char(s: *const c_char, len: c_long) -> bool {
    if len != 1 {
        return false;
    }
    let c = *s;
    (c as u8).is_ascii_alphanumeric()
        || c == '.' as c_char
        || c == ':' as c_char
        || c == '_' as c_char
}
