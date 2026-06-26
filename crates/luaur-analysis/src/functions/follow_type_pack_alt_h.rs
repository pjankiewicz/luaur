//! Node: `cxx:Function:Luau.Analysis:Analysis/src/TypePack.cpp:257:follow`
//! Source: `Analysis/src/TypePack.cpp` (TypePack.cpp:257-312, hand-ported; principal overload)

use crate::functions::get_type_pack::get;
use crate::records::internal_compiler_error::InternalCompilerError;
use crate::records::type_pack::TypePack;
use crate::type_aliases::bound_type_pack::BoundTypePack;
use crate::type_aliases::type_pack_id::TypePackId;

type Mapper = fn(*const core::ffi::c_void, TypePackId) -> TypePackId;

pub unsafe fn follow_pack_full(
    mut tp: TypePackId,
    context: *const core::ffi::c_void,
    mapper: Mapper,
) -> TypePackId {
    let advance = |ty: TypePackId| -> Option<TypePackId> {
        let mapped = mapper(context, ty);

        let btv = get::<BoundTypePack>(mapped);
        if !btv.is_null() {
            return Some((*btv).boundTo);
        }

        let pack = get::<TypePack>(mapped);
        if !pack.is_null() && (*pack).head.is_empty() {
            return (*pack).tail;
        }

        None
    };

    let mut cycle_tester: TypePackId = tp;
    if let Some(a) = advance(cycle_tester) {
        cycle_tester = a;
    } else {
        return tp;
    }

    if advance(cycle_tester).is_none() {
        return cycle_tester;
    }

    loop {
        match advance(tp) {
            Some(a1) => tp = a1,
            None => return tp,
        }

        if !cycle_tester.is_null() {
            match advance(cycle_tester) {
                Some(a2) => match advance(a2) {
                    Some(a3) => cycle_tester = a3,
                    None => cycle_tester = core::ptr::null(),
                },
                None => cycle_tester = core::ptr::null(),
            }

            if tp == cycle_tester {
                std::panic::panic_any(InternalCompilerError::new(
                    alloc::string::String::from("Luau::follow detected a TypePack cycle!!"),
                    None,
                    None,
                ));
            }
        }
    }
}

#[allow(unused_imports)]
pub use follow_pack_full as follow_type_pack_id_void_type_pack_id_item_mapper_const_void;
