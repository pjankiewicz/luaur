use crate::functions::coverage_active::coverage_active;
use luaur_compiler::records::compile_options::CompileOptions as LuauCompileOptions;

#[repr(C)]
#[allow(non_snake_case)]
pub(crate) struct GlobalOptions {
    pub(crate) optimizationLevel: i32,
    pub(crate) debugLevel: i32,
}

extern "C" {
    #[link_name = "luaur_mut"]
    pub(crate) static mut globalOptions: GlobalOptions;
}

pub fn copts() -> LuauCompileOptions {
    let mut result = LuauCompileOptions::default();

    unsafe {
        result.optimization_level = globalOptions.optimizationLevel;
        result.debug_level = globalOptions.debugLevel;
    }

    result.type_info_level = 1;
    result.coverage_level = if coverage_active() { 2 } else { 0 };

    result
}

pub(crate) mod internal {
    use super::*;

    pub fn copts_wrapper() -> LuauCompileOptions {
        copts()
    }
}
