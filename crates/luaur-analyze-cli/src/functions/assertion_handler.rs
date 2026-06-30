#[export_name = "luaur_assertion_handler"]
pub unsafe extern "C" fn assertion_handler(
    expr: *const core::ffi::c_char,
    file: *const core::ffi::c_char,
    line: core::ffi::c_int,
    _function: *const core::ffi::c_char,
) -> core::ffi::c_int {
    let file_str = unsafe { core::ffi::CStr::from_ptr(file) }.to_string_lossy();
    let expr_str = unsafe { core::ffi::CStr::from_ptr(expr) }.to_string_lossy();
    // Using libc printf equivalent via std::io::Write to stdout
    use std::io::Write;
    let _ = write!(
        std::io::stdout(),
        "{}({}): ASSERTION FAILED: {}\n",
        file_str,
        line,
        expr_str
    );
    let _ = std::io::stdout().flush();
    1
}
