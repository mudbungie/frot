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
//! [`content_signals`] therefore walks the body with chrome ([`is_chrome`])
//! and never-rendered nodes
//! (`script`/`style`/`noscript`/`template`, and any subtree this recipe does
//! not render — [`not_rendered`]) set aside, collecting the two
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
//!
//! ## Non-rendered is not content (`bl-eeb4`)
//!
//! Shells ship fallback copy the user never sees — a `<div hidden>` holding
//! "Opens in a new tab", a CSS-hidden "could not load the required files"
//! panel. Counting that text as body content is the false-`ok` direction this
//! design rejects, so [`not_rendered`] drops those subtrees: with author CSS
//! in the recipe the cascade is the visibility oracle (it carries the UA
//! `[hidden] { display: none }` rule); without it the document's own `hidden`
//! attribute is the only visibility fact there is.

use crate::css::Styles;
use crate::dom::{Document, Element, NodeId, NodeKind};
use crate::envelope::{NeedsKind, View};
use crate::fetch::header_value;

/// Transport-declared deferral: the server itself marked a success response
/// as a stand-in for the page (`docs/design/needs.md` §3). Three declarations
/// are recognized, none a body string-match:
///
/// - `Retry-After` present on the response. RFC 9110 §10.2.3 defines it for
///   503 and 3xx; on a success response it says "this body is a placeholder,
///   come back" — the shape of a bot-challenge interstitial served at 200.
/// - `cf-mitigated: challenge`, Cloudflare's documented challenge marker.
/// - `x-amzn-waf-action: challenge`, AWS WAF's documented marker — the same
///   declaration in a different vendor's spelling, measured on Amazon's 202
///   (`bl-7e34`).
///
/// The caller (`run.rs`) consults this only on the < 400 path — at >= 400
/// the error flip already reports honestly — and returns pre-parse, so a
/// declared challenge's scripts are never executed: detection stops earlier
/// than evasion could begin.
pub fn challenge(headers: &[(String, String)]) -> bool {
    header_value(headers, "retry-after").is_some()
        || ["cf-mitigated", "x-amzn-waf-action"]
            .iter()
            .any(|h| declares_challenge(headers, h))
}

/// A vendor mitigation header whose value is literally `challenge`. Both
/// vendors reuse the header for other actions (`block`, `count`), so only the
/// challenge value declares a deferral.
fn declares_challenge(headers: &[(String, String)], name: &str) -> bool {
    header_value(headers, name).is_some_and(|v| v.trim().eq_ignore_ascii_case("challenge"))
}

/// Views whose output materially depends on rendered body content.
fn view_depends_on_content(view: View) -> bool {
    matches!(view, View::Text | View::Ax | View::Links | View::Forms)
}

/// `styles` is the recipe's cascade when it has one (`--css`), else `None`;
/// it decides which subtrees render (see [`not_rendered`]).
pub fn detect(view: View, doc: &Document, styles: Option<&Styles>) -> Vec<NeedsKind> {
    if !view_depends_on_content(view) {
        return Vec::new();
    }
    let mut out = Vec::new();
    if needs_js(view, doc, styles) {
        out.push(NeedsKind::Js);
    }
    out
}

fn needs_js(view: View, doc: &Document, styles: Option<&Styles>) -> bool {
    let Some(body) = find_body(doc) else {
        return false;
    };
    has_scripts(doc) && starved(view, content_signals(doc, body, styles))
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
fn content_signals(doc: &Document, body: NodeId, styles: Option<&Styles>) -> Content {
    let mut c = Content {
        text: false,
        label: false,
    };
    collect(doc, body, styles, false, &mut c);
    c
}

/// Descend `id`, skipping non-content subtrees entirely. `sectioned` is the
/// only context the walk carries: whether an HTML sectioning ancestor
/// (`article`/`aside`/`main`/`nav`/`section`) scopes this node, which is what
/// decides a `<header>`/`<footer>` (see [`is_chrome`]).
fn collect(doc: &Document, id: NodeId, styles: Option<&Styles>, sectioned: bool, c: &mut Content) {
    let entry = doc.node(id);
    match &entry.kind {
        NodeKind::Element(el) => {
            if is_non_content(el, id, sectioned, styles) {
                return;
            }
            c.label |= has_label(el);
            let sectioned = sectioned || is_sectioning(&el.name);
            for &kid in &entry.children {
                collect(doc, kid, styles, sectioned, c);
            }
        }
        NodeKind::Text(t) => c.text |= !t.trim().is_empty(),
        NodeKind::Comment(_) | NodeKind::Doctype => {}
    }
}

/// A non-empty `alt` or `aria-label`: content a non-text view can express.
fn has_label(el: &crate::dom::Element) -> bool {
    ["alt", "aria-label"]
        .iter()
        .any(|a| el.attr(a).is_some_and(|v| !v.trim().is_empty()))
}

/// Chrome frames content without being content ([`is_chrome`]);
/// `script`/`style` and friends never render; and a subtree this recipe does
/// not render is not content either ([`not_rendered`]). Neither their elements
/// nor their text count toward the body's rendered substance, so their whole
/// subtree is skipped.
fn is_non_content(el: &Element, id: NodeId, sectioned: bool, styles: Option<&Styles>) -> bool {
    matches!(
        el.name.as_str(),
        "script" | "style" | "noscript" | "template"
    ) || is_chrome(&el.name, sectioned)
        || not_rendered(el, id, styles)
}

/// Whether this element frames the page rather than carrying its content.
/// `nav`/`aside` always do. `header`/`footer` do only at page scope: HTML-AAM
/// maps them to the `banner`/`contentinfo` landmarks *unless* a sectioning
/// element scopes them, and a scoped one is that section's own heading or
/// byline — content. Without this the rule swallows a rendered app: TodoMVC's
/// live `<section id=root><header><h1>todos</h1>…` is an app that rendered,
/// not a masthead over a dead mount (`bl-eeb4`).
fn is_chrome(name: &str, sectioned: bool) -> bool {
    match name {
        "nav" | "aside" => true,
        "header" | "footer" => !sectioned,
        _ => false,
    }
}

/// HTML sectioning content — the ancestors that scope a `<header>`/`<footer>`
/// (see [`is_chrome`]).
fn is_sectioning(name: &str) -> bool {
    matches!(name, "article" | "aside" | "main" | "nav" | "section")
}

/// Whether the recipe renders this element at all. Under `--css` the cascade
/// answers — it already folds the UA `[hidden] { display: none }` rule
/// ([`crate::dom::Element::hidden`]), author rules and inline `style=` into one
/// `display`. Without it there is no cascade, and the `hidden` attribute is the
/// document's only visibility fact.
fn not_rendered(el: &Element, id: NodeId, styles: Option<&Styles>) -> bool {
    match styles {
        Some(s) => s.display_none(id),
        None => el.hidden(),
    }
}

fn find_body(doc: &Document) -> Option<NodeId> {
    doc.find_by_tag("body").first().copied()
}

fn has_scripts(doc: &Document) -> bool {
    !doc.find_by_tag("script").is_empty()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod hidden_tests;
