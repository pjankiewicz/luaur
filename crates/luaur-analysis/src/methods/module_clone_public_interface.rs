use crate::enums::solver_mode::SolverMode;
use crate::records::builtin_types::BuiltinTypes;
use crate::records::clone_public_interface::ClonePublicInterface;
use crate::records::clone_state::CloneState;
use crate::records::internal_error::InternalError;
use crate::records::internal_error_reporter::InternalErrorReporter;
use crate::records::module::Module;
use crate::records::scope::Scope;
use crate::records::txn_log::TxnLog;
use crate::records::type_error::TypeError;
use crate::type_aliases::type_pack_id::TypePackId;
use alloc::sync::Arc;
use alloc::vec::Vec;
use luaur_ast::records::location::Location;
use luaur_common::records::dense_hash_map::DenseHashMap;

impl Module {
    /// `void Module::clonePublicInterface(NotNull<BuiltinTypes> builtinTypes, InternalErrorReporter& ice, SolverMode mode)`.
    /// Reference: `Module.cpp:299-348`.
    pub fn clone_public_interface(
        &mut self,
        builtin_types: *mut BuiltinTypes,
        _ice: &mut InternalErrorReporter,
        mode: SolverMode,
    ) {
        // C++ `CloneState cloneState{builtinTypes};` — declared (parity) but the
        // interface clone is driven by `ClonePublicInterface`'s own substitution.
        let _clone_state = CloneState {
            builtin_types,
            seen_types: DenseHashMap::new(core::ptr::null()),
            seen_type_packs: DenseHashMap::new(core::ptr::null()),
        };

        let module_scope = self.get_module_scope();
        // The C++ mutates the Scope behind the shared_ptr; mirror that by taking a
        // raw pointer to the aliased Scope object.
        let module_scope_ptr = Arc::as_ptr(&module_scope) as *mut Scope;

        let return_type: TypePackId = unsafe { (*module_scope_ptr).return_type };
        let varargpack: Option<TypePackId> = if mode == SolverMode::New {
            None
        } else {
            unsafe { (*module_scope_ptr).vararg_pack }
        };

        // C++ `TxnLog log;` — a fresh, empty transaction log.
        let log = TxnLog {
            type_var_changes: DenseHashMap::new(core::ptr::null()),
            type_pack_changes: DenseHashMap::new(core::ptr::null()),
            parent: core::ptr::null_mut(),
            owned_seen: Vec::new(),
            // Empty; lazily owns a box on first `push_seen` (freed on drop).
            shared_seen: core::ptr::null_mut(),
            owned_seen_box: None,
            radioactive: false,
        };
        let mut clone_public_interface =
            ClonePublicInterface::new(&log, builtin_types, self as *mut Module, mode);

        let return_type = clone_public_interface.clone_type_pack(return_type);

        unsafe { (*module_scope_ptr).return_type = return_type };
        if let Some(vp) = varargpack {
            let varargpack = clone_public_interface.clone_type_pack(vp);
            unsafe { (*module_scope_ptr).vararg_pack = Some(varargpack) };
        }

        unsafe {
            for (_name, tf) in (*module_scope_ptr).exported_type_bindings.iter_mut() {
                let cloned = clone_public_interface.clone_type_fun(&*tf);
                *tf = cloned;
            }
        }

        for (_name, ty) in self.declared_globals.iter_mut() {
            *ty = clone_public_interface.clone_type(*ty);
        }

        for tf in self.type_function_aliases.iter_mut() {
            let cloned = clone_public_interface.clone_type_fun(&**tf);
            **tf = cloned;
        }

        if clone_public_interface.internal_type_escaped {
            self.errors
                .push(TypeError::type_error_location_module_name_type_error_data(
                    // Not amazing but the best we can do.
                    Location::default(),
                    self.name.clone(),
                    InternalError::new(alloc::string::String::from(
                        "An internal type is escaping this module; please report this bug at \
                     https://github.com/luau-lang/luau/issues",
                    ))
                    .into(),
                ));
        }

        // Copy external stuff over to Module itself
        self.return_type = unsafe { (*module_scope_ptr).return_type };
        self.exported_type_bindings = unsafe { (*module_scope_ptr).exported_type_bindings.clone() };
    }
}
