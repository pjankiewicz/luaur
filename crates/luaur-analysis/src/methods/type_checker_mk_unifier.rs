use crate::enums::variance::Variance;
use crate::records::count_mismatch::CountMismatchContext;
use crate::records::module::Module;
use crate::records::normalizer::Normalizer;
use crate::records::scope::Scope;
use crate::records::txn_log::TxnLog;
use crate::records::type_checker::TypeChecker;
use crate::records::unifier::Unifier;
use crate::type_aliases::scope_ptr_type::ScopePtr;
use crate::type_aliases::type_or_pack_id::TypeOrPackId;
use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use luaur_ast::records::location::Location;
use luaur_common::records::dense_hash_map::DenseHashMap;

impl TypeChecker {
    pub fn mk_unifier(&mut self, scope: &ScopePtr, location: &Location) -> Unifier {
        // C++ `Unifier::tryUnify` (the public entry) resets `iterationCount = 0`
        // at the start of every top-level unification (Unifier.cpp:385/1396).
        // `iteration_count` lives in the TypeChecker-wide `unifier_state.counters`
        // shared across every unify; child unifiers (`make_child_unifier`) share it
        // so a single top-level unify accumulates correctly, but a NEW top-level
        // unify must start from zero. `mk_unifier` is the one boundary where a
        // fresh top-level Unifier is created (children bypass it), so resetting
        // here — rather than in each of the several `TypeChecker::{unify, try_unify,
        // ...}` wrappers that call the recursive `tryUnify_` directly — gives every
        // top-level unify a fresh budget, matching C++. Without it the counter
        // leaked across the whole module check and spuriously tripped
        // LuauTypeInferIterationLimit (luau_subtyping_is_np_hard).
        self.unifier_state.counters.iteration_count = 0;

        let module =
            Arc::as_ptr(self.current_module.as_ref().expect("current_module")) as *mut Module;
        let types = unsafe { &mut (*module).internal_types as *mut _ };
        self.normalizer.arena = types;
        self.normalizer.shared_state = &mut self.unifier_state;

        let normalizer_ptr: *mut Normalizer = &mut self.normalizer;
        let scope_ptr: *mut Scope = Arc::as_ptr(scope) as *mut Scope;
        // Own the seen set in a boxed Vec freed when this Unifier's log drops,
        // instead of leaking it via `Box::into_raw` on every top-level unify (the
        // leak the fuzz suite's LeakSanitizer flagged here).
        let mut seen_box: Box<Vec<(TypeOrPackId, TypeOrPackId)>> = Box::new(Vec::new());
        let shared_seen: *mut Vec<(TypeOrPackId, TypeOrPackId)> = seen_box.as_mut();

        Unifier {
            types,
            builtin_types: self.builtin_types,
            normalizer: normalizer_ptr,
            scope: scope_ptr,
            log: TxnLog {
                type_var_changes: DenseHashMap::new(core::ptr::null()),
                type_pack_changes: DenseHashMap::new(core::ptr::null()),
                parent: core::ptr::null_mut(),
                owned_seen: Vec::new(),
                shared_seen,
                owned_seen_box: Some(seen_box),
                radioactive: false,
            },
            failure: false,
            errors: Vec::new(),
            location: *location,
            variance: Variance::Covariant,
            normalize: true,
            check_inhabited: true,
            ctx: CountMismatchContext::Arg,
            shared_state: &mut self.unifier_state,
            blocked_types: alloc::vec::Vec::new(),
            blocked_type_packs: alloc::vec::Vec::new(),
            first_pack_error_pos: None,
        }
    }
}
