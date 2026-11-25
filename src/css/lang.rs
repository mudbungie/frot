//! `:lang()` and `:dir()`: the language and the direction an element
//! inherits. Measured in Chrome 158 (`bl-2a85`).
//!
//! **Language.** The nearest `lang` on the element or above it, as written
//! (an SVG element's `xml:lang` arrives here as `lang`; an HTML element's
//! `xml:lang` is ignored, as in Chrome). With none, the last
//! `<meta http-equiv=content-language>` in the document, its `content` as
//! written. `lang=""` is an unknown language and matches no range.
//! `:lang(fr)` matches `fr` and `fr-CA` in any letter case — a prefix that
//! ends at a `-`, nothing more (`zh-TW` does not match `zh-Hant-TW`). Chrome
//! takes one identifier: a string, a list or an empty argument is invalid.
//! The HTTP `Content-Language` header is not read.
//!
//! **Direction.** An HTML element's `dir` of `ltr` or `rtl` (any case) sets it;
//! `dir=auto`, or a `<bdi>` without a valid `dir`, takes it from the first
//! strongly directional character — of the value of an `<input>` or
//! `<textarea>`, else of the text inside, skipping `<bdi>`, `<script>`,
//! `<style>`, `<textarea>` and elements with a valid `dir` — and is `ltr`
//! when there is none. An `<input type=tel>` without a valid `dir` is `ltr`.
//! Anything else inherits; the root is `ltr`. `dir` on an SVG or MathML
//! element is ignored. A character is strongly right-to-left when it is a
//! letter in a Hebrew, Arabic or other right-to-left block, strongly
//! left-to-right when it is any other letter — an approximation of the
//! Unicode bidi classes that agrees on letters, which is what decides.

use super::selector::ancestors;
use crate::dom::{Document, Element, NodeId, NodeKind, WalkEvent};

/// `:lang(range)`, `range` lower-cased.
pub(super) fn lang(doc: &Document, id: NodeId, range: &str) -> bool {
    let own = chain(doc, id).find_map(|(_, e)| e.attr("lang"));
    let Some(tag) = own.map(str::to_owned).or_else(|| default(doc)) else {
        return false;
    };
    let tag = tag.to_ascii_lowercase();
    let rest = tag.strip_prefix(range);
    rest.is_some_and(|r| r.is_empty() || r.starts_with('-'))
}

/// The last `<meta http-equiv=content-language>`'s `content`.
fn default(doc: &Document) -> Option<String> {
    let mut found = None;
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(_), NodeKind::Element(e)) = (ev, &entry.kind) {
            let pragma = e.attr("http-equiv");
            if e.name == "meta"
                && pragma.is_some_and(|p| p.eq_ignore_ascii_case("content-language"))
            {
                found = e.attr("content").map(str::to_owned).or(found.take());
            }
        }
    });
    found
}

/// `:dir(rtl)`: whether the element at `id` is right-to-left.
pub(super) fn rtl(doc: &Document, id: NodeId) -> bool {
    for (n, e) in chain(doc, id) {
        if foreign(doc, n) {
            continue;
        }
        let dir = e.attr("dir").map(str::to_ascii_lowercase);
        match dir.as_deref() {
            Some("ltr") => return false,
            Some("rtl") => return true,
            Some("auto") => return auto(doc, n, e),
            _ if e.name == "bdi" => return auto(doc, n, e),
            _ if e.name == "input" && kind(e) == "tel" => return false,
            _ => {}
        }
    }
    false
}

fn kind(e: &Element) -> String {
    e.attr("type").unwrap_or_default().to_ascii_lowercase()
}

/// The direction `dir=auto` resolves to.
fn auto(doc: &Document, id: NodeId, el: &Element) -> bool {
    let text = match el.name.as_str() {
        "input" => el.attr("value").unwrap_or_default().to_owned(),
        "textarea" => doc.text_content(id),
        _ => {
            let mut text = String::new();
            gather(doc, id, &mut text);
            text
        }
    };
    text.chars().find_map(strong).unwrap_or(false)
}

/// The text below `id` that `dir=auto` reads (module doc).
fn gather(doc: &Document, id: NodeId, out: &mut String) {
    for &c in &doc.node(id).children {
        match &doc.node(c).kind {
            NodeKind::Text(t) => out.push_str(t),
            NodeKind::Element(e) if !skipped(e) => gather(doc, c, out),
            _ => {}
        }
    }
}

fn skipped(e: &Element) -> bool {
    let valid = e.attr("dir").map(str::to_ascii_lowercase);
    matches!(e.name.as_str(), "bdi" | "script" | "style" | "textarea")
        || matches!(valid.as_deref(), Some("ltr" | "rtl" | "auto"))
}

/// `Some(true)` for a strongly right-to-left character, `Some(false)` for a
/// strongly left-to-right one (module doc).
fn strong(c: char) -> Option<bool> {
    let u = c as u32;
    let rtl_block = matches!(u, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
        || matches!(u, 0x10800..=0x10FFF | 0x1E800..=0x1EFFF);
    c.is_alphabetic().then_some(rtl_block)
}

/// The element and its element ancestors, nearest first.
fn chain(doc: &Document, id: NodeId) -> impl Iterator<Item = (NodeId, &Element)> {
    doc.element(id)
        .map(|e| (id, e))
        .into_iter()
        .chain(ancestors(doc, id))
}

/// Whether the element at `id` is an SVG or MathML element: an `<svg>` or a
/// `<math>`, or inside one with no `<foreignObject>` between.
pub fn foreign(doc: &Document, id: NodeId) -> bool {
    for (n, e) in chain(doc, id) {
        match e.name.as_str() {
            "svg" | "math" => return true,
            "foreignobject" if n != id => return false,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests;
