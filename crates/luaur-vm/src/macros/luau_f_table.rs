//! C++ `extern const luau_FastFunction luauF_table[256]` (lbuiltins.h:9).
//! Source: `VM/src/lbuiltins.cpp:2495-2508,2739-2742` (hand-ported fallback)
use crate::type_aliases::luau_fast_function::luau_FastFunction;

#[allow(non_upper_case_globals)]
#[export_name = "luaur_luauF_table"]
pub static luauF_table: [luau_FastFunction; 256] =
    [Some(crate::functions::luau_f_missing::luau_f_missing); 256];
