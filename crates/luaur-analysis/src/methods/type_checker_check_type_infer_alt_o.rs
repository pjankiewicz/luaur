use crate::enums::control_flow::ControlFlow;
use crate::functions::follow_type::follow_type_id;
use crate::functions::get_mutable_type::get_mutable_type_id;
use crate::functions::get_type_alt_j::get_type_id;
use crate::functions::get_type_pack::get_type_pack_id;
use crate::records::free_type::FreeType;
use crate::records::generic_type::GenericType;
use crate::records::generic_type_pack::GenericTypePack;
use crate::records::metatable_type::MetatableType;
use crate::records::scope::Scope;
use crate::records::table_type::TableType;
use crate::records::type_checker::TypeChecker;
use crate::records::type_fun::TypeFun;
use crate::type_aliases::name_type_infer::Name;
use crate::type_aliases::scope_ptr_type_infer::ScopePtr;
use core::ffi::CStr;
use luaur_ast::records::ast_stat_type_alias::AstStatTypeAlias;

impl TypeChecker {
    pub fn check_scope_ptr_ast_stat_type_alias(
        &mut self,
        scope: &ScopePtr,
        typealias: &AstStatTypeAlias,
    ) -> ControlFlow {
        let name_cstr = unsafe { CStr::from_ptr(typealias.name.value) };

        if name_cstr.to_bytes() == b"%error-id%" || name_cstr.to_bytes() == b"typeof" {
            return ControlFlow::None;
        }

        let name: Name = name_cstr.to_string_lossy().into_owned();

        if self
            .duplicate_type_aliases
            .contains(&(typealias.exported, name.clone()))
        {
            return ControlFlow::None;
        }

        let binding = if typealias.exported {
            scope.exported_type_bindings.get(&name).cloned()
        } else {
            scope.private_type_bindings.get(&name).cloned()
        };

        let Some(binding) = binding else {
            return ControlFlow::None;
        };

        let alias_scope = self.child_scope(scope, &typealias.base.base.location);
        unsafe {
            let alias_scope_raw = alias_scope.as_ref() as *const Scope as *mut Scope;
            (*alias_scope_raw).level = scope.level.incr();

            for param in binding.type_params() {
                let generic = get_type_id::<GenericType>(param.ty);
                if !generic.is_null() {
                    (*alias_scope_raw)
                        .private_type_bindings
                        .insert((*generic).name.clone(), TypeFun::type_fun_type_id(param.ty));
                }
            }

            for param in binding.type_pack_params() {
                let generic = get_type_pack_id::<GenericTypePack>(param.tp);
                if !generic.is_null() {
                    (*alias_scope_raw)
                        .private_type_pack_bindings
                        .insert((*generic).name.clone(), param.tp);
                }
            }
        }

        let mut ty = self.resolve_type(alias_scope.clone(), unsafe { &*typealias.type_ptr });

        unsafe {
            // `getMutable` requires a followed type (it asserts the arg is not a
            // BoundType). `ty` here is the raw result of `resolve_type`, which for
            // a self-referential / chained alias (e.g. `type A = A`, or `type T =
            // Pt; type Pt = ... T ...`) is a Bound — so follow before inspecting it,
            // matching the sibling `check_scope_ptr_ast_stat_local`. (C++ Luau's
            // assert is compiled out in release, masking this; our fuzz build arms
            // it, where it aborted.)
            let table = get_mutable_type_id::<TableType>(follow_type_id(ty));
            if !table.is_null() {
                let type_params: alloc::vec::Vec<_> =
                    binding.type_params().iter().map(|param| param.ty).collect();
                let type_pack_params: alloc::vec::Vec<_> = binding
                    .type_pack_params()
                    .iter()
                    .map(|param| param.tp)
                    .collect();

                let same_tys = (*table).instantiated_type_params == type_params;
                let same_tps = (*table).instantiated_type_pack_params == type_pack_params;

                if (*table).name.is_some()
                    && ((*table).name.as_ref() != Some(&name) || !same_tys || !same_tps)
                {
                    let mut clone = (*table).clone();
                    clone.name = Some(name.clone());
                    clone.instantiated_type_params = type_params;
                    clone.instantiated_type_pack_params = type_pack_params;
                    ty = self.add_type(&clone);
                } else {
                    (*table).name = Some(name.clone());
                    (*table).instantiated_type_params = type_params;
                    (*table).instantiated_type_pack_params = type_pack_params;
                }
            } else {
                let metatable = get_mutable_type_id::<MetatableType>(follow_type_id(ty));
                if !metatable.is_null() {
                    (*metatable).syntheticName = Some(name.clone());
                }
            }

            let scope_raw = scope.as_ref() as *const Scope as *mut Scope;
            let bindings = if typealias.exported {
                &mut (*scope_raw).exported_type_bindings
            } else {
                &mut (*scope_raw).private_type_bindings
            };

            if let Some(binding) = bindings.get_mut(&name) {
                self.unify_type_id_type_id_scope_ptr_location(
                    ty,
                    binding.r#type,
                    &alias_scope,
                    &typealias.base.base.location,
                );

                let followed_binding = follow_type_id(binding.r#type);
                if !get_type_id::<FreeType>(followed_binding).is_null() {
                    binding.r#type = ty;
                } else {
                    binding.r#type = ty;
                }
            }
        }

        ControlFlow::None
    }
}
