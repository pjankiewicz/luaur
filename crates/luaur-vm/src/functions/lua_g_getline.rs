use crate::type_aliases::proto::Proto;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

#[export_name = "luaur_luaG_getline"]
pub unsafe fn luaG_getline(p: *mut Proto, pc: core::ffi::c_int) -> core::ffi::c_int {
    LUAU_ASSERT!(pc >= 0 && pc < (*p).sizecode);

    if (*p).lineinfo.is_null() {
        return 0;
    }

    let abs_index = (pc >> (*p).linegaplog2) as usize;
    let line_index = pc as usize;

    let abs_line = *((*p).abslineinfo.add(abs_index));
    let line_offset = *((*p).lineinfo.add(line_index)) as core::ffi::c_int;

    abs_line + line_offset
}
