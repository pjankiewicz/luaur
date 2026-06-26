use crate::records::txn_log::TxnLog;
use crate::type_aliases::type_or_pack_id::TypeOrPackId;
use alloc::boxed::Box;
use alloc::vec::Vec;

impl TxnLog {
    pub fn push_seen_type_or_pack_id_type_or_pack_id(
        &mut self,
        lhs: TypeOrPackId,
        rhs: TypeOrPackId,
    ) {
        if self.shared_seen.is_null() {
            // Lazily own a fresh seen set (freed on drop) instead of leaking it.
            let mut seen_box = Box::new(Vec::new());
            self.shared_seen = seen_box.as_mut() as *mut _;
            self.owned_seen_box = Some(seen_box);
        }

        let sorted_pair = if lhs > rhs { (lhs, rhs) } else { (rhs, lhs) };

        unsafe {
            (*self.shared_seen).push(sorted_pair);
        }
    }
}
