use crate::functions::add_refinement::add_refinement;
use crate::functions::baseof::baseof;
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::records::field::Field;
use crate::records::never_type::NeverType;
use crate::records::type_checker::TypeChecker;
use crate::records::union_type::UnionType;
use crate::type_aliases::l_value::{LValue, LValueMember};
use crate::type_aliases::refinement_map::RefinementMap;
use crate::type_aliases::scope_ptr_type_infer::ScopePtr;
use crate::type_aliases::type_id::TypeId;
use crate::type_aliases::type_id_predicate::TypeIdPredicate;
use luaur_ast::records::location::Location;
use luaur_common::macros::luau_assert::LUAU_ASSERT;

impl TypeChecker {
    pub fn refine_l_value(
        &mut self,
        lvalue: &LValue,
        refis: &mut RefinementMap,
        scope: ScopePtr,
        predicate: TypeIdPredicate,
    ) {
        let mut target: *const LValue = lvalue;
        // If set, we know we took the base of the lvalue path and should be walking down each option of the base's type.
        let mut key: Option<LValue> = None;

        // `std::function` is copyable in C++; `Box<dyn Fn>` is not. Wrap in an Arc so
        // we can fabricate fresh delegating predicates per call site faithfully.
        let predicate = std::sync::Arc::new(predicate);
        let make_predicate = |predicate: &std::sync::Arc<TypeIdPredicate>| -> TypeIdPredicate {
            let predicate = predicate.clone();
            Box::new(move |ty: TypeId| (predicate)(ty))
        };

        let mut ty = self.resolve_l_value_scope_ptr_l_value(scope.clone(), unsafe { &*target });
        if ty.is_none() {
            return; // Do nothing. An error was already reported.
        }

        // If the provided lvalue is a local or global, then that's without a doubt the target.
        // However, if there is a base lvalue, then we'll want that to be the target iff the base is a union type.
        let base = baseof(lvalue);
        if !base.is_null() {
            let base_ty = self.resolve_l_value_scope_ptr_l_value(scope.clone(), unsafe { &*base });
            if let Some(base_ty) = base_ty {
                if unsafe { !get_type_id::<UnionType>(follow_type_id(base_ty)).is_null() } {
                    ty = Some(base_ty);
                    target = base;
                    key = Some(lvalue.clone());
                }
            }
        }

        // If we do not have a key, it means we're not trying to discriminate anything, so it's a simple matter of just filtering for a subset.
        if key.is_none() {
            let (result, _ok) = self.filter_map(ty.unwrap(), make_predicate(&predicate));
            add_refinement(refis, unsafe { &*target }, result.unwrap());
            return;
        }

        // Otherwise, we'll want to walk each option of ty, get its index type, and filter that.
        let utv = unsafe { get_type_id::<UnionType>(follow_type_id(ty.unwrap())) };
        LUAU_ASSERT!(!utv.is_null());

        // Insertion-order dedup (not HashSet<TypeId>): the iteration order of a
        // pointer-keyed HashSet is per-instance randomized, which would make the
        // refined union's option order — and diagnostics derived from it —
        // nondeterministic. Preserving the source union's order keeps it stable.
        let mut viable_target_options: alloc::vec::Vec<TypeId> = alloc::vec::Vec::new();
        // There may be additional refinements that apply. We add those here too.
        let mut viable_child_options: alloc::vec::Vec<TypeId> = alloc::vec::Vec::new();

        let key_ref = key.as_ref().unwrap();
        for &option in unsafe { (*utv).options.iter() } {
            let discriminant_ty: Option<TypeId>;
            if let Some(field) = <Field as LValueMember>::get_if(key_ref) {
                discriminant_ty = self.get_index_type_from_type(
                    scope.clone(),
                    option,
                    &field.key,
                    &Location::default(),
                    false,
                );
            } else {
                LUAU_ASSERT!(false); // "Unhandled LValue alternative?"
                discriminant_ty = None;
            }

            let discriminant_ty = match discriminant_ty {
                Some(d) => d,
                None => return, // Do nothing. An error was already reported, as per usual.
            };

            let (result, _ok) = self.filter_map(discriminant_ty, make_predicate(&predicate));
            let result = result.unwrap();
            if unsafe { get_type_id::<NeverType>(result).is_null() } {
                if !viable_target_options.contains(&option) {
                    viable_target_options.push(option);
                }
                if !viable_child_options.contains(&result) {
                    viable_child_options.push(result);
                }
            }
        }

        let into_type = |this: &mut TypeChecker, s: &[TypeId]| -> Option<TypeId> {
            if s.is_empty() {
                return None;
            }

            // TODO: allocate UnionType and just normalize.
            let options: alloc::vec::Vec<TypeId> = s.to_vec();
            if options.len() == 1 {
                return Some(options[0]);
            }

            Some(unsafe {
                (*(alloc::sync::Arc::as_ptr(this.current_module.as_ref().unwrap())
                    as *mut crate::records::module::Module))
                    .internal_types
                    .add_type(UnionType { options })
            })
        };

        if let Some(viable_target_type) = into_type(self, &viable_target_options) {
            add_refinement(refis, unsafe { &*target }, viable_target_type);
        }

        if let Some(viable_child_type) = into_type(self, &viable_child_options) {
            add_refinement(refis, lvalue, viable_child_type);
        }
    }
}
