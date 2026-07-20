//! Capability gap detection.
//!
//! When the current recipe is insufficient to faithfully produce a
//! content-dependent view, this module reports which capabilities the page
//! wants. The envelope's `status` flips to `needs` and the listed
//! capabilities are surfaced to the caller. Two detectors feed one taxonomy
//! (`docs/design/needs.md`):
//!
//! - [`challenge`] — a **transport** fact: the server declared the response
//!   a stand-in for the page (`needs: ["human"]`, §3 of the design).
//! - [`detect`] — a **document** fact: the DOM under the recipe carries no
//!   renderable content (`needs: ["js"]`, the SPA-shell heuristic below).
//!
//! ## The signal: no rendered content behind static chrome
//!
//! A client-side-rendered app ships a scaffold — React's `<div id=root>`,
//! TodoMVC's empty `section.todoapp`, telegram's 49-empty-element sidebar
//! shell — that JavaScript later fills. Before that script runs (or if it
//! runs and renders nothing) the page's *content* is absent even though the
//! shell is dressed with static `<header>`/`<footer>`/`<nav>` boilerplate.
//! Those framing elements are **chrome**, not content, and how much empty
//! furniture the scaffold ships is noise: a lone mount `<div>` and a large
//! tree of empty boxes are the same fact — nothing renderable. (An earlier
//! guard also required "at most three elements", a proxy for "a mount
//! region" that read telegram's big-but-empty scaffold as a rendered page.)
//!
//! [`content_signals`] therefore walks the body with chrome
//! (`header`/`footer`/`nav`/`aside`) and never-rendered nodes
//! (`script`/`style`/`noscript`/`template`) set aside, collecting the two
//! ways a document can carry rendered content: **text** (a non-whitespace
//! text node) and **label** (an element with a non-empty `alt` or
//! `aria-label` — content a non-text view can express without text).
//! Starvation is view-sensitive (`docs/design/needs.md` §4): the `text` view
//! is starved without text; `ax`/`links`/`forms` are starved without text or
//! label. If scripts are present and the view is starved, the page
//! `needs: ["js"]`. The check re-runs on the post-settle DOM, so a `--js`
//! pass that leaves the app dead still trips it: the boundary moves, it
//! doesn't disappear. False `needs-js` is cheaper than false `ok`, so the
//! test errs toward flagging — the documented residual is a zero-text page
//! of labeled images, which keeps flagging the `text` view while `ax` reads
//! it honestly via labels.

use crate::dom::{Document, NodeId, NodeKind, WalkEvent};
use crate::envelope::{NeedsKind, View};
use crate::fetch::header_value;

/// Transport-declared deferral: the server itself marked a success response
/// as a stand-in for the page (`docs/design/needs.md` §3). Two declarations
/// are recognized, neither a body string-match:
///
/// - `Retry-After` present on the response. RFC 9110 §10.2.3 defines it for
///   503 and 3xx; on a success response it says "this body is a placeholder,
///   come back" — the shape of a bot-challenge interstitial served at 200.
/// - `cf-mitigated: challenge`, Cloudflare's documented challenge marker.
///
/// The caller (`run.rs`) consults this only on the < 400 path — at >= 400
/// the error flip already reports honestly — and returns pre-parse, so a
/// declared challenge's scripts are never executed: detection stops earlier
/// than evasion could begin.
pub fn challenge(headers: &[(String, String)]) -> bool {
    header_value(headers, "retry-after").is_some()
        || header_value(headers, "cf-mitigated")
            .is_some_and(|v| v.trim().eq_ignore_ascii_case("challenge"))
}

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
    if needs_js(view, doc) {
        out.push(NeedsKind::Js);
    }
    out
}

fn needs_js(view: View, doc: &Document) -> bool {
    let Some(body) = find_body(doc) else {
        return false;
    };
    has_scripts(doc) && starved(view, content_signals(doc, body))
}

/// The two ways a body can carry rendered content (module docs).
struct Content {
    text: bool,
    label: bool,
}

/// Whether `view` finds nothing to render in the body's content signals:
/// `text` can only express text; the other content views can also express an
/// `alt`/`aria-label`. There is no element count — an empty scaffold is
/// starved no matter how large (`docs/design/needs.md` §4).
fn starved(view: View, c: Content) -> bool {
    match view {
        View::Text => !c.text,
        _ => !c.text && !c.label,
    }
}

/// Collect [`Content`] over the body with chrome and never-rendered subtrees
/// set aside. An empty `alt`/`aria-label` (the decorative-image marker) is no
/// label.
fn content_signals(doc: &Document, body: NodeId) -> Content {
    let mut skip_depth = 0usize;
    let mut c = Content { text: false, label: false };
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
                NodeKind::Element(el) if has_label(el) => c.label = true,
                NodeKind::Text(t) if !t.trim().is_empty() => c.text = true,
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
    c
}

/// A non-empty `alt` or `aria-label`: content a non-text view can express.
fn has_label(el: &crate::dom::Element) -> bool {
    ["alt", "aria-label"]
        .iter()
        .any(|a| el.attr(a).is_some_and(|v| !v.trim().is_empty()))
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
