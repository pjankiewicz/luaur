//! @interface-stub
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::functions::merge::merge;
use crate::records::type_checker::TypeChecker;
use crate::records::union_type::UnionType;
use crate::type_aliases::refinement_map::RefinementMap;
use crate::type_aliases::type_id::TypeId;

impl TypeChecker {
    pub fn merge(&mut self, l: &mut RefinementMap, r: &RefinementMap) {
        let this = self as *mut TypeChecker;

        merge(l, r, &|a, b| {
            // Insertion-order dedup (not HashSet<TypeId>): a pointer-keyed HashSet
            // iterates in per-instance randomized order, which would make the merged
            // union's option order — and every diagnostic derived from it —
            // nondeterministic across runs. Preserving a-then-b order is stable.
            let mut options: alloc::vec::Vec<TypeId> = alloc::vec::Vec::new();
            let mut push = |t: TypeId| {
                if !options.contains(&t) {
                    options.push(t);
                }
            };

            unsafe {
                let followed_a = follow_type_id(a);
                let utv = get_type_id::<UnionType>(followed_a);
                if !utv.is_null() {
                    for option in &(*utv).options {
                        push(*option);
                    }
                } else {
                    push(a);
                }

                let followed_b = follow_type_id(b);
                let utv = get_type_id::<UnionType>(followed_b);
                if !utv.is_null() {
                    for option in &(*utv).options {
                        push(*option);
                    }
                } else {
                    push(b);
                }
            }

            if options.len() == 1 {
                options[0]
            } else {
                unsafe { (*this).add_type(&UnionType { options }) }
            }
        });
    }
}
