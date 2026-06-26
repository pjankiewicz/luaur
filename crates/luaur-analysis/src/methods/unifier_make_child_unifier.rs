use crate::records::txn_log::TxnLog;
use crate::records::unifier::Unifier;
use alloc::boxed::Box;
use alloc::vec::Vec;
use luaur_common::records::dense_hash_map::DenseHashMap;

impl Unifier {
    pub fn unifier_make_child_unifier(&mut self) -> Box<Unifier> {
        let parent_log: *mut TxnLog = &mut self.log;

        let u = Box::new(Unifier {
            types: self.types,
            builtin_types: self.builtin_types,
            normalizer: self.normalizer,
            scope: self.scope,
            log: TxnLog {
                type_var_changes: DenseHashMap::new(core::ptr::null()),
                type_pack_changes: DenseHashMap::new(core::ptr::null()),
                parent: parent_log,
                owned_seen: Vec::new(),
                // Child borrows the parent's seen set; it does not own/free it.
                shared_seen: self.log.shared_seen,
                owned_seen_box: None,
                radioactive: false,
            },
            failure: false,
            errors: Vec::new(),
            location: self.location,
            variance: self.variance,
            normalize: self.normalize,
            check_inhabited: self.check_inhabited,
            ctx: self.ctx,
            shared_state: self.shared_state,
            blocked_types: Vec::new(),
            blocked_type_packs: Vec::new(),
            first_pack_error_pos: None,
        });

        u
    }
}
