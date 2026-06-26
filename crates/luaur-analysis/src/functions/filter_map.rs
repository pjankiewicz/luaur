use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::records::union_type::UnionType;
use crate::type_aliases::type_id::TypeId;
use crate::type_aliases::type_id_predicate::TypeIdPredicate;
use alloc::vec::Vec;

pub fn filter_map(type_: TypeId, predicate: TypeIdPredicate) -> Vec<TypeId> {
    let type_ = unsafe { follow_type_id(type_) };

    unsafe {
        if !get_type_id::<UnionType>(type_).is_null() {
            let utv = get_type_id::<UnionType>(type_);
            // Dedupe while preserving the union's option order. The C++ original
            // uses `std::set<TypeId>` (ordered by pointer address — deterministic
            // within a run but ASLR-dependent across runs); a `HashSet<TypeId>`
            // iterates in per-instance RandomState order, making the resulting
            // union's option order — and every diagnostic derived from it —
            // nondeterministic even within a single process. Insertion-order
            // dedup is fully deterministic and keeps diagnostics stable.
            let mut options: Vec<TypeId> = Vec::new();

            for &option in (*utv).options.iter() {
                let followed_option = follow_type_id(option);
                if let Some(out) = predicate(followed_option) {
                    if !options.contains(&out) {
                        options.push(out);
                    }
                }
            }

            options
        } else if let Some(out) = predicate(type_) {
            vec![out]
        } else {
            Vec::new()
        }
    }
}
