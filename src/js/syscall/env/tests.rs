//! The environment syscalls, driven through raw `__frot_*` calls and the
//! `env.js` shims they back, for 100% Rust coverage without leaning on the JS
//! prelude (which `llvm-cov` cannot see — that is the golden suite's job).

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, Session, StyleSource};

fn sess(url: &str, ua: &str) -> Session {
    let env = Env {
        url: url.into(),
        user_agent: ua.into(),
        accept_language: "en-US,en;q=0.5".into(),
    };
    Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    )
}

#[test]
fn env_profile_feeds_navigator() {
    // The effective UA rides through to navigator.userAgent — the shim never lies
    // about who fetched (identity.md §8, I5): a caller `-H User-Agent` override is
    // the value in JS too, and appVersion is it minus the `Mozilla/` prefix.
    let s = sess("https://example.com/", "custom-ua/9");
    assert_eq!(s.eval("navigator.userAgent").unwrap(), "custom-ua/9");
    assert_eq!(s.eval("navigator.appVersion").unwrap(), "custom-ua/9");
    assert_eq!(s.eval("navigator.sendBeacon('/x', 'y')").unwrap(), "false");
    assert_eq!(
        s.eval("typeof navigator.serviceWorker").unwrap(),
        "undefined"
    );
    assert_eq!(s.eval("navigator.webdriver").unwrap(), "false");
}

/// A session with an explicit effective `Accept-Language`, to drive the
/// `navigator.language(s)` derivation from a non-default header (I5).
fn sess_lang(accept_language: &str) -> Session {
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: crate::fetch::user_agent(&[]),
        accept_language: accept_language.into(),
    };
    Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    )
}

#[test]
fn persona_serializes_the_pinned_facts_from_the_profile() {
    // The pure builder is the single derivation site; asserting its JSON directly
    // covers every field in Rust (no JS, no starvation edges). The UA/appVersion
    // come from the effective UA — here the pinned Firefox 140esr persona.
    let ua = crate::fetch::user_agent(&[]);
    let json = super::persona(&ua, "en-US,en;q=0.5");
    let v: serde_json::Value = serde_json::from_str(&json).expect("persona is valid JSON");
    assert_eq!(v["userAgent"], serde_json::Value::String(ua.clone()));
    assert_eq!(
        v["appVersion"],
        serde_json::json!(ua.strip_prefix("Mozilla/").unwrap())
    );
    assert!(
        ua.contains("rv:140.0"),
        "the pinned persona UA drives JS too"
    );
    assert_eq!(v["platform"], "Linux x86_64");
    assert_eq!(v["oscpu"], "Linux x86_64");
    assert_eq!(v["appName"], "Netscape");
    assert_eq!(v["appCodeName"], "Mozilla");
    assert_eq!(v["product"], "Gecko");
    assert_eq!(v["productSub"], "20100101");
    assert_eq!(v["vendor"], "");
    assert_eq!(v["vendorSub"], "");
    assert_eq!(v["buildID"], "20181001000000");
    assert_eq!(v["hardwareConcurrency"], 8);
    assert_eq!(v["maxTouchPoints"], 0);
    assert_eq!(v["doNotTrack"], "unspecified");
    assert_eq!(v["cookieEnabled"], true);
    assert_eq!(v["onLine"], true);
    assert_eq!(v["pdfViewerEnabled"], true);
    assert_eq!(v["colorDepth"], 24);
    assert_eq!(v["devicePixelRatio"], 1);
    assert_eq!(v["locale"], "en-US");
    assert_eq!(v["timeZone"], "UTC");
    assert_eq!(v["languages"], serde_json::json!(["en-US", "en"]));
    assert_eq!(v["language"], "en-US");
}

#[test]
fn navigator_languages_derive_from_the_effective_accept_language() {
    // Default persona header → the two-tag list Firefox reports, same source as
    // the HTTP `Accept-Language`, so they cannot disagree (identity.md §8).
    let s = sess_lang("en-US,en;q=0.5");
    assert_eq!(s.eval("navigator.languages.join(',')").unwrap(), "en-US,en");
    assert_eq!(s.eval("navigator.language").unwrap(), "en-US");
    // A caller `-H Accept-Language` override updates only these derived facts:
    // the q-weights are stripped and order preserved.
    let f = sess_lang("fr-CA,fr;q=0.8,en;q=0.5");
    assert_eq!(
        f.eval("navigator.languages.join(',')").unwrap(),
        "fr-CA,fr,en"
    );
    assert_eq!(f.eval("navigator.language").unwrap(), "fr-CA");
    // An empty header degrades to an empty list and an empty primary language
    // (the None arm of the primary-language pick).
    let e = sess_lang("");
    assert_eq!(e.eval("navigator.languages.length").unwrap(), "0");
    assert_eq!(e.eval("navigator.language").unwrap(), "");
}

#[test]
fn location_decomposes_a_parseable_url() {
    let s = sess("https://ex.com:8443/a/b?x=1#frag", "ua/1");
    let l = |expr: &str| s.eval(&format!("__frot_location().{expr}")).unwrap();
    assert_eq!(l("href"), "https://ex.com:8443/a/b?x=1#frag");
    assert_eq!(l("protocol"), "https:");
    assert_eq!(l("host"), "ex.com:8443");
    assert_eq!(l("hostname"), "ex.com");
    assert_eq!(l("port"), "8443");
    assert_eq!(l("pathname"), "/a/b");
    assert_eq!(l("search"), "?x=1");
    assert_eq!(l("hash"), "#frag");
    assert_eq!(l("origin"), "https://ex.com:8443");
    // The default port is elided from host/port; the prelude reflects href.
    let d = sess("http://plain.example/", "ua/1");
    assert_eq!(d.eval("__frot_location().host").unwrap(), "plain.example");
    assert_eq!(d.eval("__frot_location().port").unwrap(), "");
    assert_eq!(d.eval("location.href").unwrap(), "http://plain.example/");
}

#[test]
fn location_of_an_unparseable_url_keeps_href_and_empties_the_rest() {
    let s = sess("not a url", "ua/1");
    assert_eq!(s.eval("__frot_location().href").unwrap(), "not a url");
    assert_eq!(s.eval("__frot_location().protocol").unwrap(), "");
    assert_eq!(s.eval("__frot_location().origin").unwrap(), "");
    assert_eq!(s.eval("location.href").unwrap(), "not a url");
}

#[test]
fn viewport_width_is_the_layout_constant() {
    let s = sess("https://example.com/", "ua/1");
    let vw = crate::layout::VIEWPORT_WIDTH.to_string();
    assert_eq!(s.eval("String(__frot_viewport_width())").unwrap(), vw);
    // matchMedia rides the shared css::media evaluator (the same authority
    // @media blocks cascade through, js.md §7): a satisfied min-width and the
    // screen/all types match; a narrow max-width, print, and anything unknown
    // never do.
    assert_eq!(
        s.eval("matchMedia('(min-width: 800px)').matches").unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(max-width: 600px)').matches").unwrap(),
        "false"
    );
    assert_eq!(s.eval("matchMedia('print').matches").unwrap(), "false");
    assert_eq!(s.eval("matchMedia('screen').matches").unwrap(), "true");
    assert_eq!(
        s.eval("matchMedia('screen and (min-width: 800px)').matches")
            .unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(prefers-color-scheme: dark)').matches")
            .unwrap(),
        "false"
    );
}

#[test]
fn matchmedia_converts_em_and_rem_at_16px() {
    let s = sess("https://example.com/", "ua/1");
    // 16px per em/rem: 80em == 1280px == the viewport width.
    assert_eq!(
        s.eval("matchMedia('(min-width: 79em)').matches").unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(min-width: 80em)').matches").unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(min-width: 81em)').matches").unwrap(),
        "false"
    );
    assert_eq!(
        s.eval("matchMedia('(max-width: 50em)').matches").unwrap(),
        "false"
    );
    // rem behaves identically; fractional values parse; bare `width` too.
    assert_eq!(
        s.eval("matchMedia('(min-width: 40rem)').matches").unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(min-width: 79.9em)').matches").unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("matchMedia('(width: 80em)').matches").unwrap(),
        "true"
    );
    // px still works after the evaluator handoff (regression guard).
    assert_eq!(
        s.eval("matchMedia('(min-width: 960px)').matches").unwrap(),
        "true"
    );
}

#[test]
fn viewport_height_syscall_and_the_width_surface() {
    let s = sess("https://example.com/", "ua/1");
    // The new height syscall mirrors the width one (js.md §7/§8).
    let vh = crate::layout::VIEWPORT_HEIGHT.to_string();
    assert_eq!(s.eval("String(__frot_viewport_height())").unwrap(), vh);
    // window inner == outer report the fixed viewport.
    assert_eq!(s.eval("window.innerWidth").unwrap(), "1280");
    assert_eq!(s.eval("window.outerWidth").unwrap(), "1280");
    assert_eq!(s.eval("window.innerHeight").unwrap(), "720");
    assert_eq!(s.eval("window.outerHeight").unwrap(), "720");
    // documentElement is special-cased to the viewport (the root reports it).
    assert_eq!(
        s.eval("document.documentElement.clientWidth").unwrap(),
        "1280"
    );
    assert_eq!(
        s.eval("document.documentElement.clientHeight").unwrap(),
        "720"
    );
    // A normal element's clientWidth/clientHeight mirror offsetWidth/offsetHeight
    // (borderless, scrollbar-less model); body is a full-width block.
    assert_eq!(
        s.eval("document.body.clientWidth === document.body.offsetWidth")
            .unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("document.body.clientHeight === document.body.offsetHeight")
            .unwrap(),
        "true"
    );
    assert_eq!(s.eval("document.body.clientWidth").unwrap(), "1280");
}

#[test]
fn url_parse_syscall_decomposes_resolves_and_flags_validity() {
    let s = sess("https://example.com/", "ua/1");
    // No base is passed as `undefined` (the prelude's `new URL(x)` path); the
    // syscall takes two args. Absolute URL: valid + decomposed (shares
    // `location`'s decomposition).
    assert_eq!(
        s.eval("__frot_url_parse('https://e.com:8443/a?x=1#h', undefined).valid")
            .unwrap(),
        "true"
    );
    assert_eq!(
        s.eval("__frot_url_parse('https://e.com:8443/a?x=1#h', undefined).origin")
            .unwrap(),
        "https://e.com:8443"
    );
    // Base resolution rides the `url` crate's Url::join.
    assert_eq!(
        s.eval("__frot_url_parse('a/b?x=1', 'https://e.com/d/').href")
            .unwrap(),
        "https://e.com/d/a/b?x=1"
    );
    assert_eq!(
        s.eval("__frot_url_parse('a/b?x=1', 'https://e.com/d/').pathname")
            .unwrap(),
        "/d/a/b"
    );
    // Invalid spec (no base) and an unparseable base both report valid:false.
    assert_eq!(
        s.eval("__frot_url_parse('not a url', undefined).valid")
            .unwrap(),
        "false"
    );
    assert_eq!(
        s.eval("__frot_url_parse('x', 'not a url').valid").unwrap(),
        "false"
    );
}

mod more;
