use crate::records::txn_log::TxnLog;
use alloc::boxed::Box;
use alloc::vec::Vec;

impl TxnLog {
    pub fn txn_log_txn_log(&mut self, _parent: *mut TxnLog) {
        self.parent = _parent;

        if !_parent.is_null() {
            // Borrow the parent's seen set; release ours if we had one.
            self.shared_seen = unsafe { (*_parent).shared_seen };
            self.owned_seen_box = None;
        } else {
            // Own a fresh seen set (freed on drop) instead of leaking it.
            let mut seen_box = Box::new(Vec::new());
            self.shared_seen = seen_box.as_mut() as *mut _;
            self.owned_seen_box = Some(seen_box);
        }
    }
}
