//! Node: `cxx:Function:Luau.Analysis:Analysis/src/Type.cpp:79:follow`
//! Source: `Analysis/src/Type.cpp` (Type.cpp:79-141, hand-ported; the principal overload)

use crate::enums::follow_option::FollowOption;
use crate::functions::get_mutable_type::getMutable;
use crate::functions::get_type_alt_j::get;
use crate::functions::unwrap_lazy::unwrapLazy;
use crate::records::internal_compiler_error::InternalCompilerError;
use crate::records::lazy_type::LazyType;
use crate::records::table_type::TableType;
use crate::type_aliases::bound_type::BoundType;
use crate::type_aliases::type_id::TypeId;

type Mapper = fn(*const core::ffi::c_void, TypeId) -> TypeId;

#[allow(non_snake_case)]
pub unsafe fn follow_full(
    mut t: TypeId,
    followOption: FollowOption,
    context: *const core::ffi::c_void,
    mapper: Mapper,
) -> TypeId {
    let advance = |ty: TypeId| -> Option<TypeId> {
        let mapped = mapper(context, ty);

        let btv = get::<BoundType>(mapped);
        if !btv.is_null() {
            return Some((*btv).boundTo);
        }

        let ttv = get::<TableType>(mapped);
        if !ttv.is_null() {
            return (*ttv).bound_to;
        }

        let ltv = getMutable::<LazyType>(mapped);
        if !ltv.is_null() && followOption != FollowOption::DisableLazyTypeThunks {
            return Some(unwrapLazy(ltv));
        }

        None
    };

    // Null once we've determined that there is no cycle
    let mut cycle_tester: TypeId = t;
    if let Some(a) = advance(cycle_tester) {
        cycle_tester = a;
    } else {
        return t;
    }

    // Short circuit traversal for the rather common case when advance(advance(t)) == null
    if advance(cycle_tester).is_none() {
        return cycle_tester;
    }

    loop {
        match advance(t) {
            Some(a1) => t = a1,
            None => return t,
        }

        if !cycle_tester.is_null() {
            match advance(cycle_tester) {
                Some(a2) => match advance(a2) {
                    Some(a3) => cycle_tester = a3,
                    None => cycle_tester = core::ptr::null(),
                },
                None => cycle_tester = core::ptr::null(),
            }

            if t == cycle_tester {
                std::panic::panic_any(InternalCompilerError::new(
                    alloc::string::String::from("Luau::follow detected a Type cycle!!"),
                    None,
                    None,
                ));
            }
        }
    }
}

#[allow(unused_imports)]
pub use follow_full as follow_type_id_follow_option_void_type_id_item_mapper_const_void;
