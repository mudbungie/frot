//! The document base URL, pinned case by case (bl-409e).
//!
//! The pinned set is the one a real page can present: no `<base>`, a `<base>`
//! without `href`, and root-relative / path-relative / scheme-relative /
//! absolute / invalid / empty hrefs, plus multiple `<base>` elements. Each case
//! is asserted as the *absolute* URL Chrome reports for `document.baseURI` at
//! the same final URL — the fact every consumer then joins against.

use super::*;
use crate::dom::Document;

/// The document base URL as a string, for a page whose final URL is `page_url`.
fn base_of(html: &str, page_url: &str) -> String {
    resolve(base_url(&Document::parse(html), page_url).as_ref(), "")
}

/// The fixture set: `(html, page url, expected document base URL)`.
const CASES: &[(&str, &str, &str)] = &[
    // No `<base>`: the page's own final URL.
    (
        "<p>x</p>",
        "https://example.com/a/b",
        "https://example.com/a/b",
    ),
    // A `<base>` without `href` sets no base.
    (
        "<base target='_blank'>",
        "https://example.com/a/b",
        "https://example.com/a/b",
    ),
    // Root-relative — the angular.dev shape this ball was filed on. The raw `/`
    // is not a URL; resolved against the page it is the origin root.
    (
        "<base href='/'>",
        "https://example.com/a/b",
        "https://example.com/",
    ),
    // Path-relative: resolved against the page's directory, not its file.
    (
        "<base href='sub/'>",
        "https://example.com/a/b",
        "https://example.com/a/sub/",
    ),
    // Scheme-relative: the page's scheme, a new host.
    (
        "<base href='//cdn.example.com/assets/'>",
        "https://example.com/a/b",
        "https://cdn.example.com/assets/",
    ),
    // Absolute: taken as-is.
    (
        "<base href='https://cdn.example.com/v2/'>",
        "https://example.com/a/b",
        "https://cdn.example.com/v2/",
    ),
    // Invalid (a scheme with no host): unresolvable, so the page URL stands.
    (
        "<base href='http://'>",
        "https://example.com/a/b",
        "https://example.com/a/b",
    ),
    // Empty href: resolves to the document URL, exactly as a browser freezes it.
    (
        "<base href=''>",
        "https://example.com/a/b",
        "https://example.com/a/b",
    ),
    // Multiple elements: the first with an `href` wins, later ones are ignored.
    (
        "<base href='/first/'><base href='/second/'>",
        "https://example.com/a/b",
        "https://example.com/first/",
    ),
    // The first `<base>` has no `href`, so it sets nothing: the next one does.
    (
        "<base target='_top'><base href='/second/'>",
        "https://example.com/a/b",
        "https://example.com/second/",
    ),
    // An empty first href *is* an href: it wins, and means the document URL.
    (
        "<base href=''><base href='/second/'>",
        "https://example.com/a/b",
        "https://example.com/a/b",
    ),
];

#[test]
fn base_url_is_always_absolute() {
    for (html, page, want) in CASES {
        assert_eq!(&base_of(html, page), want, "html={html} page={page}");
    }
}

#[test]
fn a_redirected_page_bases_on_the_final_url() {
    // The page URL handed in is the post-redirect one, so `/` under a redirect
    // means the landing origin's root — never the requested URL's.
    assert_eq!(
        base_of("<base href='/'>", "https://landed.example.org/deep/page"),
        "https://landed.example.org/"
    );
}

#[test]
fn unparseable_page_url_has_no_base() {
    assert!(base_url(&Document::parse("<base href='/'>"), "not a url").is_none());
}

#[test]
fn resolve_without_a_base_passes_the_reference_through() {
    assert_eq!(resolve(None, "/x"), "/x");
}

#[test]
fn resolve_joins_against_the_base() {
    let base = base_url(
        &Document::parse("<base href='/app/'>"),
        "https://example.com/a/b",
    );
    assert_eq!(
        resolve(base.as_ref(), "x.png"),
        "https://example.com/app/x.png"
    );
}

#[test]
fn resolve_keeps_a_reference_the_base_cannot_join() {
    // A cannot-be-a-base base (`mailto:`) has no path to join onto.
    let base = base_url(
        &Document::parse("<base href='mailto:a@b.example'>"),
        "https://e.com/",
    );
    assert_eq!(resolve(base.as_ref(), "/x"), "/x");
}
