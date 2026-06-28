//! Node: `cxx:Method:Luau.Analysis:Analysis/src/Unifier.cpp:49:promote_type_levels_traverse`
//! Source: `Analysis/src/Unifier.cpp:23-141` (hand-ported)
//!
//! C++ `struct PromoteTypeLevels final : TypeOnceVisitor`. The visitor itself
//! does not customize traversal (unlike `FreeTypeSearcher`), so we wire it to
//! the base `GenericTypeVisitor::traverse` by implementing
//! `GenericTypeVisitorTrait`. This is what makes `promoteTypeLevels` actually
//! recurse and fire `log.changeLevel(...)`; the entry points call
//! `traverse_type_id` / `traverse_type_pack_id` rather than a single `visit`.
//!
//! The per-node `visit(...)` overrides (Unifier.cpp:49-120) are inlined here so
//! the live traversal path is self-contained and faithful to the C++ guards
//! (arena ownership, `log.is<T>` "uncommitted bound" check, table `Free`/
//! `Generic` state filter).

use crate::records::free_type::FreeType;
use crate::records::free_type_pack::FreeTypePack;
use crate::records::function_type::FunctionType;
use crate::records::generic_type_visitor::{GenericTypeVisitor, GenericTypeVisitorTrait};
use crate::records::promote_type_levels::PromoteTypeLevels;
use crate::records::table_type::TableType;
use crate::records::type_arena::TypeArena;
use crate::records::type_pack_var::TypePackVar;
use crate::type_aliases::bound_type::BoundType;
use crate::type_aliases::bound_type_pack::BoundTypePack;
use crate::type_aliases::type_id::TypeId;
use crate::type_aliases::type_pack_id::TypePackId;
use core::ffi::c_void;
use luaur_common::records::dense_hash_set::DenseHashSet;

impl GenericTypeVisitorTrait for PromoteTypeLevels {
    type Seen = DenseHashSet<*mut c_void>;

    fn visitor_base(&mut self) -> &mut GenericTypeVisitor<Self::Seen> {
        &mut self.base.base
    }

    /// `bool visit(TypeId ty) override` (Unifier.cpp:49-56).
    fn visit_type_id(&mut self, ty: TypeId) -> bool {
        unsafe {
            // Type levels of types from other modules are already global.
            if (*ty).owning_arena != self.type_arena as *mut TypeArena {
                return false;
            }
        }
        true
    }

    /// `bool visit(TypePackId tp) override` (Unifier.cpp:58-65).
    fn visit_type_pack_id(&mut self, tp: TypePackId) -> bool {
        unsafe {
            let tp_var: *const TypePackVar = tp as *const TypePackVar;
            if (*tp_var).owningArena != self.type_arena as *mut TypeArena {
                return false;
            }
        }
        true
    }

    /// `bool visit(TypeId ty, const FreeType&) override` (Unifier.cpp:67-76).
    fn visit_type_id_free_type(&mut self, ty: TypeId, _ftv: &FreeType) -> bool {
        unsafe {
            // Surprise, it's actually a BoundType that hasn't been committed
            // yet. Calling getMutable on this will trigger an assertion — and so
            // would `is::<FreeType>` below, because it goes *through* getMutable.
            // `is::<BoundType>` is the one query getMutable permits without
            // asserting, so use it to detect (and skip) the now-bound case.
            if (*self.log).txn_log_is::<BoundType, TypeId>(ty) {
                return true;
            }
            if !(*self.log).txn_log_is::<FreeType, TypeId>(ty) {
                return true;
            }
            let ft = (*self.log).txn_log_get_mutable::<FreeType, TypeId>(ty);
            self.promote(ty, ft, (*ft).level);
        }
        true
    }

    /// `bool visit(TypeId ty, const FunctionType&) override` (Unifier.cpp:78-91).
    fn visit_type_id_function_type(&mut self, ty: TypeId, _ftv: &FunctionType) -> bool {
        unsafe {
            if (*ty).owning_arena != self.type_arena as *mut TypeArena {
                return false;
            }
            // Mirror `visit_type_id_free_type`: the txn log may have bound this
            // type without committing; `is::<FunctionType>` goes through getMutable
            // and asserts on a bound type, so short-circuit on `is::<BoundType>`.
            if (*self.log).txn_log_is::<BoundType, TypeId>(ty) {
                return true;
            }
            if !(*self.log).txn_log_is::<FunctionType, TypeId>(ty) {
                return true;
            }
            let ft = (*self.log).txn_log_get_mutable::<FunctionType, TypeId>(ty);
            self.promote(ty, ft, (*ft).level);
        }
        true
    }

    /// `bool visit(TypeId ty, const TableType& ttv) override` (Unifier.cpp:93-109).
    fn visit_type_id_table_type(&mut self, ty: TypeId, ttv: &TableType) -> bool {
        use crate::enums::table_state::TableState;
        unsafe {
            if (*ty).owning_arena != self.type_arena as *mut TypeArena {
                return false;
            }

            if ttv.state != TableState::Free && ttv.state != TableState::Generic {
                return true;
            }

            // Mirror `visit_type_id_free_type`: the txn log may have bound this
            // type without committing; `is::<TableType>` goes through getMutable
            // and asserts on a bound type, so short-circuit on `is::<BoundType>`.
            if (*self.log).txn_log_is::<BoundType, TypeId>(ty) {
                return true;
            }
            if !(*self.log).txn_log_is::<TableType, TypeId>(ty) {
                return true;
            }
            let ttv_mut = (*self.log).txn_log_get_mutable::<TableType, TypeId>(ty);
            self.promote(ty, ttv_mut, (*ttv_mut).level);
        }
        true
    }

    /// `bool visit(TypePackId tp, const FreeTypePack&) override` (Unifier.cpp:111-120).
    fn visit_type_pack_id_free_type_pack(&mut self, tp: TypePackId, _ftp: &FreeTypePack) -> bool {
        unsafe {
            // Mirror the TypeId path (`visit_type_id_free_type`): the pack may
            // actually be a BoundTypePack that the txn log hasn't committed yet.
            // `getMutable`/`is::<FreeTypePack>` both go *through* getMutable and
            // assert on a bound pack; `is::<BoundTypePack>` is the one query
            // getMutable permits, so use it to detect and skip the now-bound case.
            if (*self.log).txn_log_is::<BoundTypePack, TypePackId>(tp) {
                return true;
            }
            if !(*self.log).txn_log_is::<FreeTypePack, TypePackId>(tp) {
                return true;
            }
            let ftp = (*self.log).txn_log_get_mutable::<FreeTypePack, TypePackId>(tp);
            self.promote_pack(tp, ftp, (*ftp).level);
        }
        true
    }
}
