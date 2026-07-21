//! Classifying a page `<script>` for the run queue (js.md §4.1).
//!
//! Pure over the DOM: given the document and a node id, decide whether the
//! script runs (inline vs external, classic vs module) or is skipped. The run
//! driver (`super::run_script_queue`) owns the ordering and execution; this owns
//! only the "what kind of script is this" question.

use crate::dom::{Document, NodeId, NodeKind};

use super::Session;

/// A discovered `<script>`, classified for execution (§4.1). `module` marks a
/// `type="module"` script: it is evaluated as a real ES module (imports resolve
/// through the §6 loader) rather than run as a classic script.
pub(super) enum Script {
    /// Inline body to evaluate — as a module when `module`, else a classic script.
    Inline { module: bool, body: String },
    /// External `src` (the attribute value) — fetched under §6, then run as a
    /// classic script or, when `module`, evaluated as a module (§4.2).
    External { module: bool, src: String },
    /// Non-JS `type` or `nomodule`: intentionally not run, not counted.
    Skip,
}

/// The next `<script>` (document order) not already in `done`, classified.
pub(super) fn next_script(session: &Session, done: &[NodeId]) -> Option<(NodeId, Script)> {
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
    } else if let Some(src) = el.attr("src").filter(|s| !s.is_empty()) {
        Script::External {
            module: is_module(el.attr("type")),
            src: src.to_string(),
        }
    } else {
        Script::Inline {
            module: is_module(el.attr("type")),
            body: doc.text_content(id),
        }
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
