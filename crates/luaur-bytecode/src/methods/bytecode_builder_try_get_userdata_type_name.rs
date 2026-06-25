use crate::records::bytecode_builder::BytecodeBuilder;
use luaur_common::enums::luau_bytecode_type::{
    LuauBytecodeType, LBC_TYPE_OPTIONAL_BIT, LBC_TYPE_TAGGED_USERDATA_BASE,
};

impl BytecodeBuilder {
    pub fn try_get_userdata_type_name(&self, type_: LuauBytecodeType) -> Option<&str> {
        // C++ `unsigned((type & ~LBC_TYPE_OPTIONAL_BIT) - LBC_TYPE_TAGGED_USERDATA_BASE)`: the
        // subtraction is done in (signed) int and cast to unsigned, so a non-userdata type wraps
        // to a huge index that fails the bounds check. The u16 subtraction here underflow-panicked.
        let index = ((type_.0 & !(LBC_TYPE_OPTIONAL_BIT.0)) as i32
            - LBC_TYPE_TAGGED_USERDATA_BASE.0 as i32) as u32;

        if index < self.userdata_types.len() as u32 {
            // C++ returns `userdataTypes[index].name` as a NUL-terminated
            // `const char*`. Our `name` is a Rust `String`, so return it as `&str`
            // (length-bounded). The previous version returned `name.as_ptr()`,
            // which callers then read with `CStr::from_ptr` — overrunning past the
            // un-terminated bytes into adjacent memory and appending a stray byte
            // to the type name (0x00 on macOS/arm64 by luck, 0x07/0x7f on Linux —
            // the compiler_debug_types failure).
            Some(self.userdata_types[index as usize].name.as_str())
        } else {
            None
        }
    }
}
