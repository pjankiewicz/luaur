//! Node: `cxx:Function:Luau.Analysis:Analysis/src/Type.cpp:37:unwrap_lazy`
//! Source: `Analysis/src/Type.cpp` (Type.cpp:37-55, hand-ported)

use crate::functions::get_type_alt_j::get;
use crate::records::internal_compiler_error::InternalCompilerError;
use crate::records::lazy_type::LazyType;
use crate::type_aliases::type_id::TypeId;

#[allow(non_snake_case)]
pub unsafe fn unwrapLazy(ltv: *mut LazyType) -> TypeId {
    let mut unwrapped: TypeId = (*ltv).unwrapped;

    if !unwrapped.is_null() {
        return unwrapped;
    }

    if let Some(unwrap) = (*ltv).unwrap {
        unwrap(&mut *ltv);
    }
    unwrapped = (*ltv).unwrapped;

    if unwrapped.is_null() {
        std::panic::panic_any(InternalCompilerError::new(
            alloc::string::String::from("Lazy Type didn't fill in unwrapped type field"),
            None,
            None,
        ));
    }

    if !get::<LazyType>(unwrapped).is_null() {
        std::panic::panic_any(InternalCompilerError::new(
            alloc::string::String::from("Lazy Type cannot resolve to another Lazy Type"),
            None,
            None,
        ));
    }

    unwrapped
}

#[allow(unused_imports)]
pub use unwrapLazy as unwrap_lazy;
