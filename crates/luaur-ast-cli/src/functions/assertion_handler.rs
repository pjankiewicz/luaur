//! Source: `CLI/src/Ast.cpp:18-22` (hand-ported)
//! C++ `static int assertionHandler(const char* expr, const char* file, int line, const char* function)`.
#[export_name = "luaur_assertion_handler"]
pub unsafe extern "C" fn assertion_handler(
    expr: *const core::ffi::c_char,
    file: *const core::ffi::c_char,
    line: core::ffi::c_int,
    _function: *const core::ffi::c_char,
) -> core::ffi::c_int {
    let file_str = unsafe { core::ffi::CStr::from_ptr(file) }.to_string_lossy();
    let expr_str = unsafe { core::ffi::CStr::from_ptr(expr) }.to_string_lossy();
    println!("{}({}): ASSERTION FAILED: {}", file_str, line, expr_str);
    1
}
