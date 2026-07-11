//! `--js` capability (Phase 4).
//!
//! The one capability permitted to mutate the DOM: page scripts run in an
//! embedded engine against a facade over the *same* arena everything else
//! reads (single source of truth — no mirror DOM). See `docs/design/js.md`.
//!
//! Layering (js.md §3): [`engine`] is the swappable rquickjs seam; [`syscall`]
//! installs the narrow host-function table over a shared [`Document`];
//! [`prelude`] is the JS web-API built on those calls. No `rquickjs` type
//! escapes `src/js/`, mirroring how no `markup5ever` type leaks past `dom.rs`.

pub mod engine;
mod geometry;
mod prelude;
mod syscall;

use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::dom::{Document, NodeId, NodeKind};
use engine::Engine;

pub use engine::EvalError;
pub use geometry::StyleSource;
pub use syscall::{Env, Log};

/// The `--js` execution outcome (`docs/design/js.md` §10), which the pipeline
/// maps into the envelope `js` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    /// Scripts that executed (§10 `scripts`); a script that threw still ran.
    pub scripts: u32,
    /// Counted failures (§10 `errors`): throws, refused navigations (the §7/§11
    /// counted no-ops the environment shim tallies), plus — until subfetch
    /// (4.6) — every external `src` script, which §4.2 skips-and-counts.
    pub errors: u32,
    /// Whether the run reached quiescence within the wall-clock budget (§5).
    pub settled: bool,
}

/// Run every page script against `doc` in document order (`docs/design/js.md`
/// §4), returning the post-JS document and the execution [`Report`]. Scripts a
/// script inserts join the queue (§4.3): each pass re-scans and runs the next
/// not-yet-run `<script>`, a fixpoint that naturally picks up appended ones.
/// `styles`/`env` seed the geometry cache (§8) and environment shims (§7).
///
/// The `DOMContentLoaded`/`load` lifecycle events and the virtual-clock timer
/// horizon are the event loop (§5, subtask 4.5); here a run is `settled` unless
/// a script trips the wall-clock budget, which stops the queue.
pub fn run(doc: Document, styles: StyleSource, env: Env) -> (Document, Report) {
    let session = Session::new(doc, styles, env);
    let mut done: Vec<NodeId> = Vec::new();
    let (mut scripts, mut errors, mut settled) = (0u32, 0u32, true);
    while let Some((id, script)) = next_script(&session, &done) {
        done.push(id);
        match script {
            // External `src` is fetched under §6 — until subfetch (4.6) lands
            // the fetch cannot happen, so §4.2 skips-and-counts the script.
            Script::External => errors += 1,
            // Non-JS `type` / `nomodule`: not for us (§4.1), skipped silently.
            Script::Skip => {}
            Script::Inline(body) => match session.eval(&body) {
                Ok(_) => scripts += 1,
                Err(EvalError::Exception(_)) => {
                    scripts += 1;
                    errors += 1;
                }
                Err(EvalError::Budget) => {
                    scripts += 1;
                    errors += 1;
                    settled = false;
                    break;
                }
            },
        }
    }
    // Fold the environment shim's refused-navigation tally into §10 `errors`.
    errors += session.denials();
    let report = Report {
        scripts,
        errors,
        settled,
    };
    (session.into_document(), report)
}

/// A discovered `<script>`, classified for execution (§4.1).
enum Script {
    /// Inline classic/module body to evaluate.
    Inline(String),
    /// External `src` — deferred to subfetch (4.6); skipped-and-counted (§4.2).
    External,
    /// Non-JS `type` or `nomodule`: intentionally not run, not counted.
    Skip,
}

/// The next `<script>` (document order) not already in `done`, classified.
fn next_script(session: &Session, done: &[NodeId]) -> Option<(NodeId, Script)> {
    let doc = session.document();
    let id = doc
        .find_by_tag("script")
        .into_iter()
        .find(|id| !done.contains(id))?;
    Some((id, classify(&doc, id)))
}

/// Classify a `<script>` per §4.1. `find_by_tag` only yields elements, but a
/// non-element id has nothing to run — a spec-legal-empty [`Script::Skip`],
/// not a panic.
fn classify(doc: &Document, id: NodeId) -> Script {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return Script::Skip;
    };
    if el.attr("nomodule").is_some() || !is_js_type(el.attr("type")) {
        Script::Skip
    } else if el.attr("src").is_some_and(|s| !s.is_empty()) {
        Script::External
    } else {
        Script::Inline(doc.text_content(id))
    }
}

/// Whether a `<script type>` names JavaScript (or a module) and so runs (§4.1);
/// absent/empty is classic JS. Any other MIME is not for us.
fn is_js_type(t: Option<&str>) -> bool {
    match t {
        None => true,
        Some(s) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "" | "module"
                | "text/javascript"
                | "application/javascript"
                | "text/ecmascript"
                | "application/ecmascript"
                | "text/jscript"
        ),
    }
}

/// A JS execution session bound to one document. It owns the engine, shares the
/// arena with the syscall closures for the mutable JS window (js.md §2), and
/// captures `console` output. Constructing it installs the syscall table and
/// evaluates the prelude; [`Session::eval`] then runs page scripts.
pub struct Session {
    engine: Engine,
    doc: syscall::SharedDoc,
    console: syscall::Console,
    denials: syscall::Denials,
}

impl Session {
    /// Bind `doc` to a fresh engine, install the syscall table, and load the
    /// prelude. `styles` is the geometry cache's styling policy (js.md §8): the
    /// pipeline passes [`StyleSource::Authored`] under `--css`, else
    /// [`StyleSource::Bare`]. `env` carries the static facts the §7 environment
    /// shims (navigator/location/matchMedia) derive from.
    pub fn new(doc: Document, styles: StyleSource, env: Env) -> Self {
        let engine = Engine::new();
        let doc = Rc::new(RefCell::new(doc));
        let console = Rc::new(RefCell::new(Vec::new()));
        let geo = Rc::new(RefCell::new(geometry::Geometry::new(styles)));
        let denials = Rc::new(RefCell::new(0));
        syscall::install(&engine, doc.clone(), console.clone(), geo, env, denials.clone());
        Session {
            engine,
            doc,
            console,
            denials,
        }
    }

    /// Evaluate a page script, draining microtasks, and coerce its value to a
    /// string (the engine's smoke surface; the event loop lands in subtask 4.5).
    pub fn eval(&self, src: &str) -> Result<String, EvalError> {
        self.engine.eval(src)
    }

    /// Borrow the post-mutation document (the pipeline consumes this after
    /// settle, js.md §9).
    pub fn document(&self) -> Ref<'_, Document> {
        self.doc.borrow()
    }

    /// Consume the session and reclaim the post-JS document. Dropping the engine
    /// first releases the syscall closures' shared handles, so the arena is
    /// solely owned again (js.md §2: mutation ends at settle).
    pub fn into_document(self) -> Document {
        let Session { engine, doc, .. } = self;
        drop(engine);
        Rc::into_inner(doc)
            .expect("engine drop leaves the arena solely owned")
            .into_inner()
    }

    /// The `console` lines captured so far, in emission order.
    pub fn console(&self) -> Ref<'_, Vec<Log>> {
        self.console.borrow()
    }

    /// Navigations the shim refused (js.md §7/§11) — the counted-no-op tally the
    /// wiring layer folds into the envelope `js.errors` count (§10).
    pub fn denials(&self) -> u32 {
        *self.denials.borrow()
    }
}

#[cfg(test)]
mod tests;
