//! The declarative refresh — `<meta http-equiv=refresh>` as a navigation.
//!
//! A zero-delay document refresh is the markup spelling of a redirect: a
//! browser lands on the target unattended, with no script and no human. frot
//! follows it as part of the initial navigation (`run.rs`; decided 2026-11-17,
//! `bl-c262`), so this module answers one question: *where does this document
//! immediately send the navigation, if anywhere?*
//!
//! The grammar and the choice of element are HTML's "shared declarative refresh
//! steps" (the `http-equiv` *refresh* pragma directive), followed literally:
//!
//! - the **first `<meta>` whose refresh parses and resolves** claims the
//!   document ("will declaratively refresh"); later ones are ignored, and one
//!   that fails to parse claims nothing;
//! - it may sit anywhere in the document — `<body>` counts as `<head>` does;
//! - a `<meta>` inside `<noscript>` is **not an element** to a scripting browser
//!   (the reference; the parser frot shares with it reads `<noscript>` content
//!   as text), so it never refreshes — the "enable JavaScript" detour is not
//!   taken, with or without `--js`;
//! - a missing URL means the document's own URL (a reload);
//! - a delay above zero is not followed: the browser shows this page first, so
//!   this page is the impression.
//!
//! Three targets a browser does not load are likewise not followed: one that
//! differs from the page only by a fragment (a scroll, not a fetch), a
//! non-`http(s)` scheme (`javascript:`, `data:`, `mailto:` leave the page in
//! place), and `file:` from a remote page (remote content never causes local
//! reads — the rule `run/gather.rs` holds for stylesheets).

use crate::dom::{Document, NodeKind, WalkEvent};
use url::Url;

/// The absolute URL the document at `page_url` immediately refreshes to, or
/// `None` when it stays put. Relative targets resolve against the document base
/// URL ([`crate::base`]), the one resolution authority.
pub fn target(doc: &Document, page_url: &str) -> Option<String> {
    let (page, base) = Url::parse(page_url)
        .ok()
        .zip(crate::base::base_url(doc, page_url))?;
    let (immediate, to) = contents(doc).iter().find_map(|content| {
        let (immediate, reference) = parse(content)?;
        let to = match reference {
            Some(r) => base.join(r).ok()?,
            None => page.clone(),
        };
        Some((immediate, to))
    })?;
    let loads = match to.scheme() {
        "http" | "https" => true,
        "file" => page.scheme() == "file",
        _ => false,
    };
    let scroll = to.fragment().is_some() && same_document(&to, &page);
    (immediate && loads && !scroll).then(|| to.to_string())
}

/// Whether two URLs name one document — equal with fragments excluded.
fn same_document(a: &Url, b: &Url) -> bool {
    let (mut a, mut b) = (a.clone(), b.clone());
    a.set_fragment(None);
    b.set_fragment(None);
    a == b
}

/// The `content` of every refresh pragma in tree order: a `<meta>` whose
/// `http-equiv` is `refresh` (ASCII case-insensitive).
fn contents(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    doc.walk(None, &mut |ev, entry| {
        if let (WalkEvent::Enter(_), NodeKind::Element(el)) = (ev, &entry.kind) {
            let pragma = el.name == "meta"
                && el
                    .attr("http-equiv")
                    .is_some_and(|v| v.eq_ignore_ascii_case("refresh"));
            if let Some(content) = el.attr("content").filter(|_| pragma) {
                out.push(content.to_string());
            }
        }
    });
    out
}

const WS: [char; 5] = [' ', '\t', '\n', '\x0c', '\r'];

/// Parse a refresh `content` value into (delay is zero, URL reference if any),
/// or `None` when the value is not a refresh at all. Step for step the spec's
/// parser: `[ws] digits [digits-and-dots] [ws] [;|,] [ws] [url [ws] = [ws]]
/// [quote] reference [quote…]`.
fn parse(content: &str) -> Option<(bool, Option<&str>)> {
    let s = content.trim_start_matches(WS);
    let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let time = &s[..s.len() - rest.len()];
    if time.is_empty() && !rest.starts_with('.') {
        return None;
    }
    let immediate = time.bytes().all(|b| b == b'0');
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
    if rest.is_empty() {
        return Some((immediate, None));
    }
    if !rest.starts_with([';', ',']) && !rest.starts_with(WS) {
        return None;
    }
    let rest = rest.trim_start_matches(WS);
    let rest = rest.strip_prefix([';', ',']).unwrap_or(rest);
    let rest = rest.trim_start_matches(WS);
    if rest.is_empty() {
        return Some((immediate, None));
    }
    Some((immediate, Some(reference(rest))))
}

/// The URL reference inside the tail of a refresh value: past an optional
/// `url =` label and an optional opening quote, up to that quote's next
/// occurrence. A tail that only *starts* like the label (`u`, `ur`, `url` with
/// no `=`) is itself the reference, as the spec's parser has it.
fn reference(tail: &str) -> &str {
    let labelled = tail
        .get(..3)
        .filter(|l| l.eq_ignore_ascii_case("url"))
        .and_then(|_| tail[3..].trim_start_matches(WS).strip_prefix('='))
        .map(|r| r.trim_start_matches(WS));
    // An unlabelled tail is quote-stripped only when it does not open with
    // `u`/`U` (the spec's "skip quotes" jump); it cannot open with a quote
    // then, so stripping it is the same no-op either way.
    let r = labelled.unwrap_or(tail);
    match r.chars().next().filter(|c| matches!(c, '"' | '\'')) {
        Some(q) => r[1..].split(q).next().unwrap_or(""),
        None => r,
    }
}

#[cfg(test)]
mod tests;
