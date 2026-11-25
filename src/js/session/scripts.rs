//! The [`Session`]'s side of the script queue (js.md §4, `bl-a86b`): which
//! scripts the parser left, the one gate each passes, and the inserted ones
//! waiting for a task.

use super::Session;
use crate::dom::NodeId;
use crate::js::script::{self, Script};

impl Session {
    /// The `<script>`s the parser left in the document, in document order —
    /// read once, before any runs, and each marked started, so no insertion
    /// runs one before the script phase reaches it, and a script a script
    /// inserts is never mistaken for one of these: it has its own path
    /// (`syscall::insert`).
    pub(in crate::js) fn parser_scripts(&self) -> Vec<NodeId> {
        let mut doc = self.doc.borrow_mut();
        let ids = doc.find_by_tag("script");
        ids.iter().for_each(|&id| doc.start(id));
        ids
    }

    /// [`script::prepare`] for one of the [`parser_scripts`](Self::parser_scripts):
    /// the script `id` if it may run now.
    pub(in crate::js) fn prepare_parsed(&self, id: NodeId) -> Option<Script> {
        script::prepare(&mut self.doc.borrow_mut(), id, &self.page_url, true)
    }

    /// The oldest inserted external or module script waiting for its task.
    pub(in crate::js) fn next_inserted(&self) -> Option<(NodeId, Script)> {
        self.pending.borrow_mut().pop_front()
    }

    /// Inline scripts the insertion hook ran, and how many threw.
    pub(in crate::js) fn inserted_tally(&self) -> (u32, u32) {
        let c = &self.counters;
        (c.scripts.get(), c.threw.get())
    }

    /// Warm the §6 cache concurrently with the initial external scripts (bl-08f6):
    /// a preload-scanner pass over the parsed document so the source-ordered
    /// script queue finds each external `src` already frozen instead of blocking
    /// on it one round trip at a time. Discovery releases the document borrow
    /// before the cache's parallel dispatch, which rides — and is charged to —
    /// the run's `NET_BUDGET_MS` network budget, and its byte pool.
    pub(in crate::js) fn warm_initial_scripts(&self) {
        let reqs = script::initial_externals(&self.doc.borrow(), &self.page_url);
        self.subfetch.borrow_mut().warm(&reqs);
    }
}
