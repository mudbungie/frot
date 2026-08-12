//! The document base URL — the one authority every consumer resolves against.
//!
//! HTML gives a page a single *document base URL*: the first `<base>` element
//! carrying an `href` attribute, that href **resolved against the document's own
//! URL** (the final URL after redirects); with no such element, or an href that
//! does not resolve, the document URL itself. So `<base href="/">` on
//! `https://example.com/a/b` is `https://example.com/` — a raw href is not a
//! base, and joining a link against the unresolved `/` cannot work.
//!
//! `--out links`/`forms`/`meta`, the `--css` stylesheet gather, and `--js`
//! script/module discovery all resolve through here, so they cannot disagree
//! about what a page's `<base>` means.

use crate::dom::{Document, NodeKind, WalkEvent};
use url::Url;

/// The document base URL of `doc` for a page whose final URL is `page_url`.
///
/// `None` only when `page_url` itself does not parse (a `file`-less test stub, a
/// bogus caller argument): there is then no absolute anchor at all, and
/// [`resolve`] passes references through untouched rather than inventing one.
pub fn base_url(doc: &Document, page_url: &str) -> Option<Url> {
    let page = Url::parse(page_url).ok()?;
    Some(match first_base_href(doc) {
        Some(href) => page.join(&href).unwrap_or(page),
        None => page,
    })
}

/// The `href` of the first `<base>` element that has one — the only `<base>`
/// HTML honours; later ones are ignored, and one without `href` sets no base.
/// An *empty* href is still an href: it resolves to the document URL, which is
/// exactly what a browser freezes.
fn first_base_href(doc: &Document) -> Option<String> {
    let mut found: Option<String> = None;
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(_), NodeKind::Element(el)) = (ev, &entry.kind) {
            if found.is_none() && el.name == "base" {
                found = el.attr("href").map(str::to_string);
            }
        }
    });
    found
}

/// Resolve `href` against `base` into an absolute URL, falling back to the raw
/// `href` when there is no base or the join fails. An impression reports what it
/// found; it never fabricates a URL it could not compute.
pub fn resolve(base: Option<&Url>, href: &str) -> String {
    base.and_then(|b| b.join(href).ok())
        .map(|u| u.to_string())
        .unwrap_or_else(|| href.to_string())
}

#[cfg(test)]
mod tests;
