use crate::records::data_flow_graph_builder::DataFlowGraphBuilder;
use crate::records::def::Def;
use crate::records::dfg_scope::DfgScope;
use crate::type_aliases::def_id_def::DefId;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

impl DataFlowGraphBuilder {
    /// `void DataFlowGraphBuilder::joinProps(DfgScope* result, const DfgScope& a, const DfgScope& b)`.
    /// Reference: `DataFlowGraph.cpp:246-294`.
    ///
    /// Borrow-safety note: the `while`-loop visitor calls `join(scope, scope, whileScope)`,
    /// so `result` and `a` are the **same** `DfgScope`. The faithful C++ mutates
    /// `result->props` while iterating `a->props` (and reads `scope->props` inside
    /// `joinProps`' lambda while holding a reference into it) — in Rust that is `&`/`&mut`
    /// aliasing UB: even without a rehash the optimizer assumes the `&mut` is unique and
    /// miscompiles, corrupting the `props` map's `Vec` header (observed as a later
    /// `find` indexing an empty `data`). We therefore **snapshot** `a`'s and `b`'s props
    /// into owned values first, and have `phinodify` build each merged entry in a local
    /// `BTreeMap` before writing it back — exactly the pattern `DfgScope::inherit` uses.
    /// The observable result is identical to the C++ (`lookup_def_id_string` is only
    /// consulted when the in-progress entry has no value for the key, so a deferred
    /// write-back cannot change its answer).
    pub fn join_props(&mut self, result: *mut DfgScope, a: &DfgScope, b: &DfgScope) {
        let def_arena = self.def_arena;

        // Owned snapshots so no borrow of `a`/`b`/`result` props is held across the
        // mutations of `result.props` below (result may alias `a`).
        let a_props: Vec<(DefId, BTreeMap<String, *const Def>)> =
            a.props.iter().map(|(k, v)| (*k, v.clone())).collect();
        let b_props: Vec<(DefId, BTreeMap<String, *const Def>)> =
            b.props.iter().map(|(k, v)| (*k, v.clone())).collect();
        let b_lookup = |def: DefId| -> Option<&BTreeMap<String, *const Def>> {
            b_props.iter().find(|(k, _)| *k == def).map(|(_, v)| v)
        };

        // C++ lambda `phinodify`: merges per-key defs of `a`/`b` into `scope->props[parent]`.
        // Builds the merged entry in a local map, then writes it back — never holds a
        // `&mut` into `scope.props` across the `lookup_def_id_string` read of `scope.props`.
        let phinodify = |scope: *mut DfgScope,
                         a_props: &BTreeMap<String, *const Def>,
                         b_props: &BTreeMap<String, *const Def>,
                         parent: DefId| unsafe {
            let mut p: BTreeMap<String, *const Def> =
                (*scope).props.find(&parent).cloned().unwrap_or_default();

            for (k, def_a) in a_props.iter() {
                let merged = if let Some(it) = b_props.get(k) {
                    (*def_arena).phi_def_id_def_id(*it, *def_a)
                } else if let Some(it) = p.get(k).copied() {
                    (*def_arena).phi_def_id_def_id(it, *def_a)
                } else if let Some(def2) = (*scope).lookup_def_id_string(parent, k) {
                    (*def_arena).phi_def_id_def_id(def2, *def_a)
                } else {
                    *def_a
                };
                p.insert(k.clone(), merged);
            }

            for (k, def_b) in b_props.iter() {
                if a_props.get(k).is_some() {
                    continue;
                }
                let merged = if let Some(it) = p.get(k).copied() {
                    (*def_arena).phi_def_id_def_id(it, *def_b)
                } else if let Some(def2) = (*scope).lookup_def_id_string(parent, k) {
                    (*def_arena).phi_def_id_def_id(def2, *def_b)
                } else {
                    *def_b
                };
                p.insert(k.clone(), merged);
            }

            *(*scope).props.get_or_insert(parent) = p;
        };

        unsafe {
            for (def, a1) in a_props.iter() {
                (*result).props.try_insert(*def, BTreeMap::new());
                if let Some(a2) = b_lookup(*def) {
                    phinodify(result, a1, a2, *def);
                } else if let Some(a2) = (*result).props.find(def) {
                    let a2 = a2.clone();
                    phinodify(result, a1, &a2, *def);
                }
            }

            for (def, a1) in b_props.iter() {
                (*result).props.try_insert(*def, BTreeMap::new());
                if a_props.iter().any(|(k, _)| k == def) {
                    continue;
                } else if let Some(a2) = (*result).props.find(def) {
                    let a2 = a2.clone();
                    phinodify(result, a1, &a2, *def);
                }
            }
        }
    }
}
