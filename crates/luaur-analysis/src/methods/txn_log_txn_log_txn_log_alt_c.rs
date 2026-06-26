use crate::records::txn_log::TxnLog;
use crate::type_aliases::type_or_pack_id::TypeOrPackId;
use alloc::vec::Vec;

impl TxnLog {
    pub fn txn_log_vector_pair_type_or_pack_id_type_or_pack_id(
        &mut self,
        shared_seen: *mut Vec<(TypeOrPackId, TypeOrPackId)>,
    ) {
        // Adopt a borrowed seen set; release ours if we owned one.
        self.shared_seen = shared_seen;
        self.owned_seen_box = None;
    }
}
