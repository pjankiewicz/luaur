//! Node: `cxx:Function:Luau.VM:VM/src/ldebug.cpp:630:lua_debugtrace`
//! Source: `VM/src/ldebug.cpp:630-677` (hand-ported)

use crate::functions::append::append;
use crate::functions::lua_getinfo::lua_getinfo;
use crate::records::lua_debug::LuaDebug;
use crate::type_aliases::lua_state::lua_State;

static mut BUF: [core::ffi::c_char; 4096] = [0; 4096];

/// Write `s` into `buf` as a NUL-terminated C string, truncating if needed
/// (the pure-Rust stand-in for the original `snprintf` calls, which have no
/// symbol to bind on `wasm32-unknown-unknown` and trapped in the browser).
unsafe fn write_c_str(buf: &mut [core::ffi::c_char], s: &str) {
    let n = s.len().min(buf.len() - 1);
    core::ptr::copy_nonoverlapping(s.as_ptr() as *const core::ffi::c_char, buf.as_mut_ptr(), n);
    buf[n] = 0;
}

#[allow(non_snake_case)]
pub unsafe fn lua_debugtrace(L: *mut lua_State) -> *const core::ffi::c_char {
    const LIMIT1: core::ffi::c_int = 10;
    const LIMIT2: core::ffi::c_int = 10;

    let depth: core::ffi::c_int = (*L).ci.offset_from((*L).base_ci) as core::ffi::c_int;
    let mut offset: usize = 0;

    let mut ar: LuaDebug = core::mem::zeroed();

    let mut level: core::ffi::c_int = 0;
    while lua_getinfo(L, level, c"sln".as_ptr(), &mut ar as *mut LuaDebug) != 0 {
        if !ar.short_src.is_null() {
            offset = append(BUF.as_mut_ptr(), BUF.len(), offset, ar.short_src);
        }

        if ar.currentline > 0 {
            let mut line: [core::ffi::c_char; 32] = [0; 32];
            write_c_str(&mut line, &alloc::format!(":{}", ar.currentline));

            offset = append(BUF.as_mut_ptr(), BUF.len(), offset, line.as_ptr());
        }

        if !ar.name.is_null() {
            offset = append(BUF.as_mut_ptr(), BUF.len(), offset, c" function ".as_ptr());
            offset = append(BUF.as_mut_ptr(), BUF.len(), offset, ar.name);
        }

        offset = append(BUF.as_mut_ptr(), BUF.len(), offset, c"\n".as_ptr());

        if depth > LIMIT1 + LIMIT2 && level == LIMIT1 - 1 {
            let mut skip: [core::ffi::c_char; 32] = [0; 32];
            write_c_str(
                &mut skip,
                &alloc::format!("... (+{} frames)\n", depth - LIMIT1 - LIMIT2),
            );

            offset = append(BUF.as_mut_ptr(), BUF.len(), offset, skip.as_ptr());

            level = depth - LIMIT2 - 1;
        }

        level += 1;
    }

    luaur_common::macros::luau_assert::LUAU_ASSERT!(offset < BUF.len());
    BUF[offset] = 0;

    BUF.as_ptr()
}
