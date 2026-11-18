//! The declarative refresh, pinned case by case (bl-08d8): the content grammar
//! of HTML's "shared declarative refresh steps", which `<meta>` claims the
//! document, and which targets a browser actually loads.

use super::*;
use crate::dom::Document;

const PAGE: &str = "https://example.com/a/b?q=1";

fn meta(content: &str) -> String {
    format!("<meta http-equiv=\"refresh\" content=\"{content}\">")
}

fn target_of(html: &str, page_url: &str) -> Option<String> {
    target(&Document::parse(html), page_url)
}

/// What a `content` value parses to: `(delay is zero, URL reference)`.
type Parsed = Option<(bool, Option<&'static str>)>;

/// `(content, parsed)`.
const GRAMMAR: &[(&str, Parsed)] = &[
    // The astro.build shape, and the label in any case with whitespace around `=`.
    ("0;url=/en/", Some((true, Some("/en/")))),
    ("0; URL = /en/", Some((true, Some("/en/")))),
    ("0;uRl\t=\n/en/", Some((true, Some("/en/")))),
    // `,` separates as `;` does; so does bare whitespace; leading whitespace skips.
    (" \t0 , /en/", Some((true, Some("/en/")))),
    ("0 /en/", Some((true, Some("/en/")))),
    // Quotes: the reference ends at the opening quote's next occurrence, or at
    // the end of the value when it never closes; unlabelled references too.
    ("0;url='/en/' x", Some((true, Some("/en/")))),
    ("0;url=&quot;/a'b&quot;c", Some((true, Some("/a'b")))),
    ("0;url='/en/", Some((true, Some("/en/")))),
    ("0;'/en/'", Some((true, Some("/en/")))),
    // A tail that only starts like the label is itself the reference.
    ("0;u", Some((true, Some("u")))),
    ("0;url", Some((true, Some("url")))),
    ("0;url /x", Some((true, Some("url /x")))),
    ("0;urn:x", Some((true, Some("urn:x")))),
    ("0;aa\u{e9}", Some((true, Some("aa\u{e9}")))),
    // An empty reference is still a reference (it resolves to the base).
    ("0;url=", Some((true, Some("")))),
    // No reference at all: a reload.
    ("0", Some((true, None))),
    ("0;", Some((true, None))),
    ("0 ; ", Some((true, None))),
    // The delay: integer part only; fractions are discarded, `.5` is zero.
    ("00;url=/x", Some((true, Some("/x")))),
    ("0.9;url=/x", Some((true, Some("/x")))),
    (".5;url=/x", Some((true, Some("/x")))),
    ("5;url=/x", Some((false, Some("/x")))),
    ("010", Some((false, None))),
    // Not a refresh: no delay, a signed delay, junk after the delay.
    ("", None),
    ("url=/x", None),
    ("-1;url=/x", None),
    ("0x", None),
    ("0.5.x;url=/x", None),
];

#[test]
fn the_content_grammar_follows_the_spec_parser() {
    for (content, want) in GRAMMAR {
        let html = meta(content);
        let doc = Document::parse(&html);
        let got = contents(&doc);
        assert_eq!(got.len(), 1, "content {content:?} is not one pragma");
        assert_eq!(parse(&got[0]), *want, "content {content:?}");
    }
}

/// `(html, page url, where the navigation goes)`.
const TARGETS: &[(&str, &str, Option<&str>)] = &[
    // Relative, against the page; absolute and cross-origin as written.
    (
        "<meta http-equiv=refresh content='0;url=/en/'>",
        PAGE,
        Some("https://example.com/en/"),
    ),
    (
        "<meta http-equiv=refresh content='0;url=c'>",
        PAGE,
        Some("https://example.com/a/c"),
    ),
    (
        "<meta http-equiv=refresh content='0;url=https://other.example/x'>",
        PAGE,
        Some("https://other.example/x"),
    ),
    // Against the document base when a `<base href>` sets one — even an empty
    // reference, which names the base itself.
    (
        "<base href='/docs/'><meta http-equiv=refresh content='0;url=start'>",
        PAGE,
        Some("https://example.com/docs/start"),
    ),
    (
        "<base href='/docs/'><meta http-equiv=refresh content='0;url='>",
        PAGE,
        Some("https://example.com/docs/"),
    ),
    // No reference: the document's own URL — a reload, `<base>` or not.
    (
        "<base href='/docs/'><meta http-equiv=refresh content='0'>",
        PAGE,
        Some(PAGE),
    ),
    // `http-equiv` matches in any case; `<body>` counts as `<head>` does.
    (
        "<META HTTP-EQUIV='ReFrEsH' CONTENT='0;url=/en/'>",
        PAGE,
        Some("https://example.com/en/"),
    ),
    (
        "<body><p>moved</p><meta http-equiv=refresh content='0;url=/en/'></body>",
        PAGE,
        Some("https://example.com/en/"),
    ),
    // A fragment rides along to a different document…
    (
        "<meta http-equiv=refresh content='0;url=/en/#install'>",
        PAGE,
        Some("https://example.com/en/#install"),
    ),
    // …but a target that is this document plus a fragment is a scroll.
    ("<meta http-equiv=refresh content='0;url=#top'>", PAGE, None),
    (
        "<meta http-equiv=refresh content='0'>",
        "https://example.com/a#here",
        None,
    ),
    // A delay above zero shows this page first.
    ("<meta http-equiv=refresh content='5;url=/en/'>", PAGE, None),
    // The first refresh that parses and resolves claims the document: a delayed
    // one blocks a later immediate one; a malformed or unresolvable one does not.
    (
        "<meta http-equiv=refresh content='5;url=/slow'>\
         <meta http-equiv=refresh content='0;url=/fast'>",
        PAGE,
        None,
    ),
    (
        "<meta http-equiv=refresh content='soon'>\
         <meta http-equiv=refresh content='0;url=http://[bad'>\
         <meta http-equiv=refresh content='0;url=/en/'>",
        PAGE,
        Some("https://example.com/en/"),
    ),
    // Not a refresh pragma at all.
    (
        "<meta http-equiv=content-type content='0;url=/en/'>",
        PAGE,
        None,
    ),
    ("<meta name=refresh content='0;url=/en/'>", PAGE, None),
    ("<meta http-equiv=refresh>", PAGE, None),
    ("<link http-equiv=refresh content='0;url=/en/'>", PAGE, None),
    ("<p>plain</p>", PAGE, None),
    // Inside `<noscript>` (head or body) and `<template>` there is no element.
    (
        "<head><noscript><meta http-equiv=refresh content='0;url=/nojs'></noscript></head>",
        PAGE,
        None,
    ),
    (
        "<body><noscript><meta http-equiv=refresh content='0;url=/nojs'></noscript></body>",
        PAGE,
        None,
    ),
    (
        "<template><meta http-equiv=refresh content='0;url=/t'></template>",
        PAGE,
        None,
    ),
    // Schemes a browser does not load from a refresh leave the page in place.
    (
        "<meta http-equiv=refresh content='0;url=javascript:void(0)'>",
        PAGE,
        None,
    ),
    (
        "<meta http-equiv=refresh content='0;url=mailto:a@example.com'>",
        PAGE,
        None,
    ),
    (
        "<meta http-equiv=refresh content='0;url=data:text/html,x'>",
        PAGE,
        None,
    ),
    // Remote content never causes a local read; a local page may go either way.
    (
        "<meta http-equiv=refresh content='0;url=file:///etc/passwd'>",
        PAGE,
        None,
    ),
    (
        "<meta http-equiv=refresh content='0;url=next.html'>",
        "file:///tmp/site/index.html",
        Some("file:///tmp/site/next.html"),
    ),
    (
        "<meta http-equiv=refresh content='0;url=https://example.com/'>",
        "file:///tmp/site/index.html",
        Some("https://example.com/"),
    ),
    // No absolute anchor, no target.
    (
        "<meta http-equiv=refresh content='0;url=/en/'>",
        "not a url",
        None,
    ),
];

#[test]
fn the_target_is_what_a_browser_would_load() {
    for (html, page, want) in TARGETS {
        assert_eq!(
            target_of(html, page).as_deref(),
            *want,
            "html {html:?} at {page}"
        );
    }
}
