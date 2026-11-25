//! The [`Session`]'s side of the script queue (js.md §4, `bl-a86b`): how a
//! script or a driver call is evaluated, in which window (`bl-b34e`), which
//! scripts the parser left, the one gate each passes, and the inserted ones
//! waiting for a task.

use super::Session;
use crate::dom::NodeId;
use crate::js::script::{self, Script};
use crate::js::syscall::Window;
use crate::js::EvalError;

impl Session {
    /// Evaluate one of frot's own event-loop driver calls inside the armed
    /// bounds (§5), draining microtasks — the [`super::run`] path's host
    /// primitive, strict like the prelude it calls into.
    pub fn run_task(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval_armed(src)
    }

    /// [`run_task`](Self::run_task) in a frame's realm, or the top's (`bl-b34e`).
    pub fn run_task_in(&self, realm: Option<usize>, src: &str) -> Result<String, EvalError> {
        self.engine.eval_in(realm, src, true)
    }

    /// Evaluate one page **classic script** — the element `id`'s — inside the
    /// armed bounds (§5). Sloppy mode unless the source says otherwise, which is
    /// what a browser does with a classic script (js.md §4.1);
    /// [`run_task`](Self::run_task) is frot's own code and stays strict.
    ///
    /// `id` is what `document.currentScript` reports while this body runs. The
    /// prior value is restored on the way out whatever the outcome — a throw, a
    /// budget trip, or a nested evaluation — because the restore is the host's,
    /// not something the script could skip.
    /// `window` is the frame it was inserted into, `None` the top (`bl-b34e`).
    pub fn run_script(
        &self,
        window: Option<&Window>,
        id: NodeId,
        src: &str,
    ) -> Result<String, EvalError> {
        let current = window.map_or(&self.current, |w| &w.current);
        let prior = current.replace(Some(id));
        let out = self.engine.eval_in(window.map(|w| w.realm), src, false);
        current.set(prior);
        out
    }

    /// Evaluate `src` as an ES module named `name` (its URL) inside the armed
    /// bounds (§5), draining microtasks — the [`super::run`] path's module
    /// primitive. Imports resolve/load through the §6 loader; failures surface as
    /// [`EvalError`] exactly like [`run_task`](Self::run_task).
    pub fn run_module(
        &self,
        realm: Option<usize>,
        name: &str,
        src: &str,
    ) -> Result<String, EvalError> {
        self.engine.eval_module(realm, name, src)
    }

    /// The `<script>`s the parser left in the document, in document order —
    /// read once, before any runs, and each marked started, so no insertion
    /// runs one before the script phase reaches it, and a script a script
    /// inserts is never mistaken for one of these: it has its own path
    /// (`syscall::insert`).
    pub(in crate::js) fn parser_scripts(&self) -> Vec<NodeId> {
        let mut doc = self.doc.borrow_mut();
        let ids = doc.find_by_tag("script");
        // The parser's scripts: started, and not "force async" (`bl-f9f2`).
        ids.iter().for_each(|&id| {
            doc.start(id);
            doc.unforce(id);
        });
        ids
    }

    /// [`script::prepare`] for one of the [`parser_scripts`](Self::parser_scripts):
    /// the script `id` if it may run now.
    pub(in crate::js) fn prepare_parsed(&self, id: NodeId) -> Option<Script> {
        script::prepare(&mut self.doc.borrow_mut(), id, &self.page_url, true)
    }

    /// The oldest inserted external or module script waiting for its task, and
    /// the frame window it was inserted into (`None`: the top).
    pub(in crate::js) fn next_inserted(&self) -> Option<(NodeId, Script, Option<Window>)> {
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
