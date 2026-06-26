use crate::records::pending_type::PendingType;
use crate::records::pending_type_pack::PendingTypePack;
use crate::type_aliases::type_or_pack_id::TypeOrPackId;
use alloc::boxed::Box;
use alloc::vec::Vec;
use luaur_common::records::dense_hash_map::DenseHashMap;

#[derive(Debug)]
pub struct TxnLog {
    pub(crate) type_var_changes:
        DenseHashMap<*const crate::records::r#type::Type, Box<PendingType>>,
    pub(crate) type_pack_changes:
        DenseHashMap<*const crate::records::type_pack_var::TypePackVar, Box<PendingTypePack>>,
    pub(crate) parent: *mut TxnLog,
    pub(crate) owned_seen: Vec<(TypeOrPackId, TypeOrPackId)>,
    /// The active "seen pairs" set. Points either at this log's own storage
    /// (`owned_seen_box`, below) or — for a child/derived log — at a parent's.
    pub(crate) shared_seen: *mut Vec<(TypeOrPackId, TypeOrPackId)>,
    /// Heap storage for `shared_seen` when THIS log owns it. C++ uses a member
    /// `std::vector` whose destructor frees it; the Rust port originally did
    /// `Box::into_raw(Box::new(Vec::new()))` and never freed it — a leak of one
    /// Vec per top-level unification (caught by the fuzz suite's LeakSanitizer in
    /// `mk_unifier`). A boxed Vec gives a stable address that survives the log
    /// being moved/returned, and being owned here it is freed on drop. `None`
    /// when this log borrows a parent's set (`shared_seen` then aliases it).
    pub(crate) owned_seen_box: Option<Box<Vec<(TypeOrPackId, TypeOrPackId)>>>,
    pub(crate) radioactive: bool,
}

impl Clone for TxnLog {
    fn clone(&self) -> Self {
        // Deep-copy the owned seen storage so the clone owns (and later frees) its
        // own box, with `shared_seen` re-pointed into it. A borrowing log (no box)
        // clones the borrowed pointer as-is. The previous `#[derive(Clone)]` copied
        // `shared_seen` as a raw pointer into the *original's* box — fine for the
        // overload-resolution clones that only read changes, but it must not be the
        // owner, or the box would be double-freed / aliased.
        match &self.owned_seen_box {
            Some(b) => {
                let mut nb = b.clone();
                let sp: *mut Vec<(TypeOrPackId, TypeOrPackId)> = nb.as_mut();
                TxnLog {
                    type_var_changes: self.type_var_changes.clone(),
                    type_pack_changes: self.type_pack_changes.clone(),
                    parent: self.parent,
                    owned_seen: self.owned_seen.clone(),
                    shared_seen: sp,
                    owned_seen_box: Some(nb),
                    radioactive: self.radioactive,
                }
            }
            None => TxnLog {
                type_var_changes: self.type_var_changes.clone(),
                type_pack_changes: self.type_pack_changes.clone(),
                parent: self.parent,
                owned_seen: self.owned_seen.clone(),
                shared_seen: self.shared_seen,
                owned_seen_box: None,
                radioactive: self.radioactive,
            },
        }
    }
}
