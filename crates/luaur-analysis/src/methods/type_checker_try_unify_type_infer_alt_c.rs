use crate::records::type_checker::TypeChecker;
use crate::type_aliases::error_vec::ErrorVec;
use crate::type_aliases::scope_ptr_type_infer::ScopePtr;
use crate::type_aliases::type_pack_id::TypePackId;
use luaur_ast::records::location::Location;

impl TypeChecker {
    /// C++ `ErrorVec TypeChecker::tryUnify(TypePackId subTp, TypePackId superTp,
    /// const ScopePtr& scope, const Location& location)` (TypeInfer.cpp).
    ///
    /// This unifies two type *packs*. The previous implementation reinterpret-cast
    /// the `TypePackId`s (`*const TypePackVar`) to `TypeId` (`*const Type`) and ran
    /// *type* unification on them — a raw-pointer type confusion: `follow` then
    /// read a `TypePackVar` as a `Type`, whose `TypeVariant` discriminant is
    /// garbage. That is layout-dependent UB (issue #6): some toolchains tolerate
    /// the bogus enum tag, others SIGSEGV. Mirror the `TypeId` overload exactly,
    /// but drive the Unifier's *pack* unify so the pointee types stay honest.
    pub fn try_unify_type_pack_id_type_pack_id_scope_ptr_location(
        &mut self,
        sub_ty: TypePackId,
        super_ty: TypePackId,
        scope: ScopePtr,
        location: &Location,
    ) -> ErrorVec {
        // `mk_unifier` resets the shared iteration counter (as the public
        // `Unifier::tryUnify` does), so call the recursive `tryUnify_` directly.
        let mut state = self.mk_unifier(&scope, location);

        state.try_unify_type_pack_id_type_pack_id_bool(sub_ty, super_ty, false);

        if state.errors.is_empty() {
            state.log.commit();
        }

        state.errors
    }
}
