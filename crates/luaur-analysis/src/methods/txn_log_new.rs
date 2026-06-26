use crate::records::txn_log::TxnLog;
use alloc::boxed::Box;
use alloc::vec::Vec;
use luaur_common::records::dense_hash_map::DenseHashMap;

impl TxnLog {
    pub fn new() -> Self {
        // Own the seen set in a boxed Vec (stable address, freed on drop) instead
        // of leaking it via `Box::into_raw`.
        let mut seen_box = Box::new(Vec::new());
        let shared_seen = seen_box.as_mut() as *mut _;
        Self {
            type_var_changes: DenseHashMap::new(core::ptr::null()),
            type_pack_changes: DenseHashMap::new(core::ptr::null()),
            parent: core::ptr::null_mut(),
            owned_seen: Vec::new(),
            shared_seen,
            owned_seen_box: Some(seen_box),
            radioactive: false,
        }
    }
}
