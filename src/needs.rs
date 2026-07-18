//! Capability gap detection.
//!
//! When the current recipe (Phase 0 = no `--css`, no `--js`) is insufficient
//! to faithfully produce a content-dependent view, [`detect`] reports which
//! capabilities the page wants. The envelope's `status` flips to `needs` and
//! the listed capabilities are surfaced to the caller.
//!
//! Phase 1 lights up the `js` capability via a coarse SPA-shell heuristic.
//!
//! ## The signal: a starved mount region behind static chrome
//!
//! A client-side-rendered app ships an empty mount container — React's
//! `<div id=root>`, TodoMVC's `<section class=todoapp>` — that JavaScript later
//! fills. Before that script runs (or if it runs and renders nothing) the
//! page's *content* is absent even though the shell is dressed with a static
//! `<header>`, `<footer>`, `<nav>`, or an SEO title. Those framing elements are
//! **chrome**, not content: an earlier heuristic that only looked for a
//! literally empty `<body>` read such shells as `ok` and emitted an essentially
//! empty impression — the exact "silently degraded result that looks complete"
//! VISION forbids. Real shells (todomvc: empty `section.todoapp` + a static
//! `footer.info`; excalidraw: a `<header>` masthead + empty root `<div>`) carry
//! that boilerplate, so a body-empty test never fired.
//!
//! [`content_starved`] therefore measures the body with chrome
//! (`header`/`footer`/`nav`/`aside`) and never-rendered nodes
//! (`script`/`style`/`noscript`/`template`) set aside. If scripts are present
//! and what remains carries **no text** and **at most three elements** — a lone
//! mount region, not a filled page and not a large scaffold of empty boxes —
//! the content the view needs isn't there and the page `needs: ["js"]`. The
//! check re-runs on the post-settle DOM, so a `--js` pass that leaves the app
//! dead still trips it: the boundary moves, it doesn't disappear. False
//! `needs-js` is cheaper than false `ok`, so the test errs toward flagging —
//! any real body text (a `<p>`, a list item, a table cell, or content the app
//! renders into its mount) clears it.

use crate::dom::{Document, NodeId, NodeKind, WalkEvent};
use crate::envelope::{NeedsKind, View};

/// Views whose output materially depends on rendered body content.
fn view_depends_on_content(view: View) -> bool {
    matches!(
        view,
        View::Text | View::Ax | View::Links | View::Forms
    )
}

pub fn detect(view: View, doc: &Document) -> Vec<NeedsKind> {
    if !view_depends_on_content(view) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if needs_js(doc) {
        out.push(NeedsKind::Js);
    }
    out
}

fn needs_js(doc: &Document) -> bool {
    let Some(body) = find_body(doc) else {
        return false;
    };
    has_scripts(doc) && content_starved(doc, body)
}

/// True when the body — with chrome and never-rendered subtrees set aside —
/// holds no text and at most three elements: a bare mount region rather than a
/// rendered page. See the module docs for the rationale.
fn content_starved(doc: &Document, body: NodeId) -> bool {
    let mut skip_depth = 0usize;
    let mut elements = 0usize;
    let mut has_text = false;
    doc.walk(Some(body), &mut |ev, entry| match ev {
        WalkEvent::Enter(_) => {
            if let NodeKind::Element(el) = &entry.kind {
                if is_non_content(&el.name) {
                    skip_depth += 1;
                    return;
                }
            }
            if skip_depth > 0 {
                return;
            }
            match &entry.kind {
                NodeKind::Element(el) if el.name != "body" => elements += 1,
                NodeKind::Text(t) if !t.trim().is_empty() => has_text = true,
                _ => {}
            }
        }
        WalkEvent::Exit(_) => {
            if let NodeKind::Element(el) = &entry.kind {
                if is_non_content(&el.name) {
                    skip_depth -= 1;
                }
            }
        }
    });
    !has_text && elements <= 3
}

/// Chrome frames content without being content; `script`/`style` and friends
/// never render. Neither their elements nor their text count toward the body's
/// rendered substance, so their whole subtree is skipped.
fn is_non_content(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "noscript"
            | "template"
            | "header"
            | "footer"
            | "nav"
            | "aside"
    )
}

fn find_body(doc: &Document) -> Option<NodeId> {
    doc.find_by_tag("body").first().copied()
}

fn has_scripts(doc: &Document) -> bool {
    !doc.find_by_tag("script").is_empty()
}

#[cfg(test)]
mod tests;
