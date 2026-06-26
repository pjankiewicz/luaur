use crate::enums::solver_mode::SolverMode;
use crate::enums::variance::Variance;
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::records::any_type::AnyType;
use crate::records::builtin_types::BuiltinTypes;
use crate::records::internal_error_reporter::InternalErrorReporter;
use crate::records::module::Module;
use crate::records::normalizer::Normalizer;
use crate::records::pending_type::PendingType;
use crate::records::pending_type_pack::PendingTypePack;
use crate::records::r#type::Type;
use crate::records::scope::Scope;
use crate::records::subtyping::Subtyping;
use crate::records::type_arena::TypeArena;
use crate::records::type_check_limits::TypeCheckLimits;
use crate::records::type_function_runtime::TypeFunctionRuntime;
use crate::records::type_pack_var::TypePackVar;
use crate::records::unifier::Unifier;
use crate::records::unifier_shared_state::UnifierSharedState;
use crate::records::union_type::UnionType;
use crate::type_aliases::type_id::TypeId;
use crate::type_aliases::type_pack_id::TypePackId;
use crate::type_aliases::type_pack_variant::TypePackVariant;
use crate::type_aliases::type_variant::TypeVariant;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use luaur_ast::records::location::Location;
use luaur_common::records::dense_hash_map::DenseHashMap;
use luaur_common::records::dense_hash_set::DenseHashSet;
use luaur_common::records::dense_hash_table::DenseDefault;
use luaur_common::FInt;

impl DenseDefault for Box<PendingType> {
    fn dense_default() -> Self {
        Box::new(PendingType {
            pending: Type::new(TypeVariant::Any(AnyType::default())),
            dead: true,
        })
    }
}

impl DenseDefault for Box<PendingTypePack> {
    fn dense_default() -> Self {
        Box::new(PendingTypePack {
            pending: TypePackVar {
                ty: TypePackVariant::Error(crate::records::unifiable::Error::<TypePackId>::new()),
                persistent: false,
                owningArena: core::ptr::null_mut(),
            },
        })
    }
}

fn empty_seen_type_pack_set() -> crate::type_aliases::seen_type_pack_set::SeenTypePackSet {
    let type_seen: DenseHashMap<
        (TypeId, TypeId),
        bool,
        crate::records::type_id_pair_hash::TypeIdPairHash,
    > = DenseHashMap::new((core::ptr::null(), core::ptr::null()));
    unsafe { core::mem::transmute(type_seen) }
}

/// C++ `checkTypeMatch(...)`.
pub fn check_type_match(
    module: &Module,
    sub_ty: TypeId,
    super_ty: TypeId,
    scope: *mut Scope,
    type_arena: *mut TypeArena,
    builtin_types: *mut BuiltinTypes,
) -> bool {
    let sub_ty = unsafe { follow_type_id(sub_ty) };
    let super_ty = unsafe { follow_type_id(super_ty) };

    if let Some(super_union) = unsafe { get_type_id::<UnionType>(super_ty).as_ref() } {
        return super_union.options.iter().any(|&option| {
            check_type_match(module, sub_ty, option, scope, type_arena, builtin_types)
        });
    }

    if let Some(sub_union) = unsafe { get_type_id::<UnionType>(sub_ty).as_ref() } {
        return sub_union.options.iter().all(|&option| {
            check_type_match(module, option, super_ty, scope, type_arena, builtin_types)
        });
    }

    let mut ice_reporter = InternalErrorReporter {
        on_internal_error: None,
        module_name: String::new(),
    };
    let mut unifier_state =
        UnifierSharedState::unifier_shared_state(&mut ice_reporter as *mut InternalErrorReporter);

    let solver_mode = if module.checked_in_new_solver {
        SolverMode::New
    } else {
        SolverMode::Old
    };

    let mut normalizer = Normalizer {
        cached_normals: alloc::collections::BTreeMap::new(),
        cached_intersections: alloc::collections::BTreeMap::new(),
        cached_unions: alloc::collections::BTreeMap::new(),
        cached_type_ids: alloc::collections::BTreeMap::new(),
        cached_is_inhabited: DenseHashMap::new(core::ptr::null()),
        cached_is_inhabited_intersection: DenseHashMap::new((core::ptr::null(), core::ptr::null())),
        fuel: None,
        arena: core::ptr::null_mut(),
        builtin_types: core::ptr::null_mut(),
        shared_state: core::ptr::null_mut(),
        cache_inhabitance: false,
        solver_mode,
    };
    unsafe {
        normalizer.normalizer_type_arena_not_null_builtin_types_not_null_unifier_shared_state_solver_mode_bool(
            type_arena,
            builtin_types,
            &mut unifier_state as *mut UnifierSharedState,
            solver_mode,
            false,
        );
    }

    if module.checked_in_new_solver {
        let mut limits = TypeCheckLimits::default();
        unsafe { Arc::increment_strong_count(scope) };
        let root_scope = unsafe { Arc::from_raw(scope) };
        let mut type_function_runtime = TypeFunctionRuntime {
            ice: ice_reporter.clone(),
            limits: limits.clone(),
            type_arena: Default::default(),
            type_pack_arena: Default::default(),
            state: (core::ptr::null_mut(), None),
            initialized: DenseHashSet::new(core::ptr::null_mut()),
            allow_evaluation: true,
            root_scope,
            messages: Vec::new(),
            runtime_builder: core::ptr::null_mut(),
        };

        unifier_state.counters.recursion_limit = FInt::LuauTypeInferRecursionLimit.get() as i32;
        unifier_state.counters.iteration_limit = FInt::LuauTypeInferIterationLimit.get() as i32;

        let mut subtyping = Subtyping {
            builtin_types: core::ptr::null_mut(),
            arena: core::ptr::null_mut(),
            normalizer: core::ptr::null_mut(),
            type_function_runtime: core::ptr::null_mut(),
            ice_reporter: core::ptr::null_mut(),
            limits: TypeCheckLimits::default(),
            unique_types: core::ptr::null(),
            seen_types: DenseHashMap::new((core::ptr::null(), core::ptr::null())),
            seen_packs: empty_seen_type_pack_set(),
            result_cache: DenseHashMap::new((core::ptr::null(), core::ptr::null())),
        };
        unsafe {
            subtyping.subtyping_not_null_builtin_types_not_null_type_arena_not_null_normalizer_not_null_type_function_runtime_not_null_internal_error_reporter(
                builtin_types,
                type_arena,
                &mut normalizer as *mut Normalizer,
                &mut type_function_runtime as *mut TypeFunctionRuntime,
                &mut ice_reporter as *mut InternalErrorReporter,
            );
        }

        let result =
            unsafe { subtyping.is_subtype_type_id_type_id_not_null_scope(sub_ty, super_ty, scope) };

        result.is_subtype
    } else {
        let location = Location::default();
        let mut unifier = Unifier {
            types: type_arena,
            builtin_types,
            normalizer: &mut normalizer as *mut Normalizer,
            scope,
            log: crate::records::txn_log::TxnLog {
                type_var_changes: DenseHashMap::new(core::ptr::null()),
                type_pack_changes: DenseHashMap::new(core::ptr::null()),
                parent: core::ptr::null_mut(),
                owned_seen: Vec::new(),
                // Empty; lazily owns a box on first `push_seen` (freed on drop).
                shared_seen: core::ptr::null_mut(),
                owned_seen_box: None,
                radioactive: false,
            },
            failure: false,
            errors: Vec::new(),
            location,
            variance: Variance::Covariant,
            normalize: true,
            check_inhabited: true,
            ctx: crate::records::count_mismatch::CountMismatchContext::Arg,
            shared_state: &mut unifier_state as *mut UnifierSharedState,
            blocked_types: Vec::new(),
            blocked_type_packs: Vec::new(),
            first_pack_error_pos: None,
        };

        // Cost of normalization can be too high for autocomplete response time requirements
        unifier.normalize = false;
        unifier.check_inhabited = false;

        unifier_state.counters.recursion_limit = FInt::LuauTypeInferRecursionLimit.get() as i32;
        unifier_state.counters.iteration_limit = FInt::LuauTypeInferIterationLimit.get() as i32;

        let errors = unsafe { unifier.can_unify_type_id_type_id(sub_ty, super_ty) };
        errors.is_empty()
    }
}
