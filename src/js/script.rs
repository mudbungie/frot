//! Classifying a page `<script>` for the run queue (js.md §4.1).
//!
//! Pure over the DOM: given the document and a node id, decide whether the
//! script runs (inline vs external, classic vs module) or is skipped. The run
//! driver (`super::queue::run_script_queue`) and the insertion hook
//! (`super::syscall::insert`) own the ordering and execution; this owns only
//! the "may this script run, and what kind is it" question.

use url::Url;

use crate::base;
use crate::dom::{Document, NodeId, NodeKind};
use crate::fetch::Intent;

use super::Session;

/// A discovered `<script>`, classified for execution (§4.1). `module` marks a
/// `type="module"` script: it is evaluated as a real ES module (imports resolve
/// through the §6 loader) rather than run as a classic script.
pub enum Script {
    /// Inline body to evaluate — as a module when `module`, else a classic script.
    Inline { module: bool, body: String },
    /// External `src`, already resolved against the document base URL — fetched
    /// under §6, then run as a classic script or, when `module`, evaluated as a
    /// module (§4.2).
    External { module: bool, src: String },
}

/// The request intent an external `<script>` fetches under (§4.1): a module
/// graph edge vs a classic script. One mapping, shared by the discovery warm
/// (bl-08f6) and the serial execution path (`super::run_external`).
pub(super) fn external_intent(module: bool) -> Intent {
    if module {
        Intent::Module
    } else {
        Intent::ClassicScript
    }
}

/// The initial external `<script src>` (classic + module) in document order —
/// the preload-scanner set the run warms concurrently before draining the queue
/// (bl-08f6). Only scripts statically present in the parsed document: a script a
/// script inserts later is not here, so it is never speculatively prefetched
/// (js.md §6 / the task's explicit non-goal). Inline and non-JS scripts have no
/// resource to fetch and are skipped.
pub(super) fn initial_externals(doc: &Document, page_url: &str) -> Vec<(String, Intent)> {
    let base = base::base_url(doc, page_url);
    doc.find_by_tag("script")
        .into_iter()
        .filter_map(|id| match classify(doc, id, base.as_ref())? {
            Script::External { module, src } => Some((src, external_intent(module))),
            Script::Inline { .. } => None,
        })
        .collect()
}

/// HTML's "prepare the script element" (js.md §4, `bl-a86b`) — the one gate
/// every script passes, parsed or inserted, whenever it arrives: `None` when
/// `id` will not run now (already started, not in the document, an empty
/// inline body with no `src`, or a type that is not JavaScript), else the
/// classified script, which is marked started so it never runs twice. An empty
/// or non-JS script stays unstarted, as in Firefox: given text, a `src` or a
/// JS `type` later, it can still run. The base is the *current* document's, so
/// a `<base>` a script inserted moves what later scripts fetch.
///
/// The `parser` asks for its own scripts, which the script phase marked
/// started before running any (`Session::parser_scripts`): in a browser a
/// script the parser has not reached does not exist yet, so nothing an earlier
/// script does to it — moving it, giving it text — may run it early.
pub(super) fn prepare(
    doc: &mut Document,
    id: NodeId,
    page_url: &str,
    parser: bool,
) -> Option<Script> {
    let is_script = doc.element(id).is_some_and(|e| e.name == "script");
    if !is_script || (doc.started(id) && !parser) || !doc.connected(id) {
        return None;
    }
    let base = base::base_url(doc, page_url);
    let script = match classify(doc, id, base.as_ref())? {
        Script::Inline { body, .. } if body.is_empty() => return None,
        s => s,
    };
    doc.start(id);
    Some(script)
}

/// The base an *inline* `type="module"` script's imports resolve against (§4.1):
/// the document base URL, or the page URL when there is no absolute one to
/// derive it from. Held in no field — the document is the fact, so it is asked
/// each time.
pub(super) fn module_base(session: &Session) -> String {
    base::base_url(&session.document(), session.page_url())
        .map_or_else(|| session.page_url().to_string(), |u| u.to_string())
}

/// Classify a `<script>` per §4.1, resolving an external `src` against the
/// document base URL so the fetch hits what a browser would — `None` for a
/// non-JS `type` or `nomodule`, which is not for us. `find_by_tag` only yields
/// elements, but a non-element id has nothing to run: `None`, not a panic.
fn classify(doc: &Document, id: NodeId, base: Option<&Url>) -> Option<Script> {
    let NodeKind::Element(el) = &doc.node(id).kind else {
        return None;
    };
    if el.attr("nomodule").is_some() || !is_js_type(el.attr("type")) {
        None
    } else if let Some(src) = el.attr("src").filter(|s| !s.is_empty()) {
        Some(Script::External {
            module: is_module(el.attr("type")),
            src: base::resolve(base, src),
        })
    } else {
        Some(Script::Inline {
            module: is_module(el.attr("type")),
            body: doc.text_content(id),
        })
    }
}

/// Whether a `<script type>` selects ES-module evaluation (§4.1) — the exact
/// `"module"` keyword (case-insensitive), the one type that is not a classic
/// script.
fn is_module(t: Option<&str>) -> bool {
    t.is_some_and(|s| s.trim().eq_ignore_ascii_case("module"))
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

#[cfg(test)]
mod tests;
