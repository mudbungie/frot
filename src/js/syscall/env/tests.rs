//! The environment syscalls, driven through raw `__frot_*` calls and the
//! `env.js` shims they back, for 100% Rust coverage without leaning on the JS
//! prelude (which `llvm-cov` cannot see — that is the golden suite's job).

use crate::dom::Document;
use crate::js::{Env, Session, StyleSource};

fn sess(url: &str, ua: &str) -> Session {
    let env = Env {
        url: url.into(),
        user_agent: ua.into(),
        headers: Vec::new(),
    };
    Session::new(Document::parse("<html><body></body></html>"), StyleSource::Bare, env)
}

#[test]
fn env_ua_feeds_navigator() {
    let s = sess("https://example.com/", "custom-ua/9");
    assert_eq!(s.eval("__frot_env_ua()").unwrap(), "custom-ua/9");
    assert_eq!(s.eval("navigator.userAgent").unwrap(), "custom-ua/9");
    assert_eq!(s.eval("navigator.sendBeacon('/x', 'y')").unwrap(), "false");
    assert_eq!(s.eval("typeof navigator.serviceWorker").unwrap(), "undefined");
    assert_eq!(s.eval("navigator.webdriver").unwrap(), "false");
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
    // matchMedia rides it: a satisfied min-width matches, a narrow max-width and
    // a featureless query never do (js.md §7).
    assert_eq!(s.eval("matchMedia('(min-width: 800px)').matches").unwrap(), "true");
    assert_eq!(s.eval("matchMedia('(max-width: 600px)').matches").unwrap(), "false");
    assert_eq!(s.eval("matchMedia('print').matches").unwrap(), "false");
}

#[test]
fn url_parse_syscall_decomposes_resolves_and_flags_validity() {
    let s = sess("https://example.com/", "ua/1");
    // No base is passed as `undefined` (the prelude's `new URL(x)` path); the
    // syscall takes two args. Absolute URL: valid + decomposed (shares
    // `location`'s decomposition).
    assert_eq!(s.eval("__frot_url_parse('https://e.com:8443/a?x=1#h', undefined).valid").unwrap(), "true");
    assert_eq!(
        s.eval("__frot_url_parse('https://e.com:8443/a?x=1#h', undefined).origin").unwrap(),
        "https://e.com:8443"
    );
    // Base resolution rides the `url` crate's Url::join.
    assert_eq!(
        s.eval("__frot_url_parse('a/b?x=1', 'https://e.com/d/').href").unwrap(),
        "https://e.com/d/a/b?x=1"
    );
    assert_eq!(s.eval("__frot_url_parse('a/b?x=1', 'https://e.com/d/').pathname").unwrap(), "/d/a/b");
    // Invalid spec (no base) and an unparseable base both report valid:false.
    assert_eq!(s.eval("__frot_url_parse('not a url', undefined).valid").unwrap(), "false");
    assert_eq!(s.eval("__frot_url_parse('x', 'not a url').valid").unwrap(), "false");
}

#[test]
fn url_prelude_builds_urls_and_searchparams() {
    let s = sess("https://example.com/", "ua/1");
    // Acceptance: base resolution + searchParams.get, and the plain components.
    let u = "new URL('a/b?x=1', 'https://e.com/d/')";
    assert_eq!(s.eval(&format!("{u}.searchParams.get('x')")).unwrap(), "1");
    assert_eq!(s.eval(&format!("{u}.href")).unwrap(), "https://e.com/d/a/b?x=1");
    assert_eq!(s.eval(&format!("{u}.pathname")).unwrap(), "/d/a/b");
    assert_eq!(s.eval(&format!("{u}.origin")).unwrap(), "https://e.com");
    assert_eq!(s.eval(&format!("{u}.toString()")).unwrap(), "https://e.com/d/a/b?x=1");
    // Invalid URL throws a TypeError (unlike `location`, which empties).
    assert_eq!(
        s.eval("try { new URL('not a url'); 'no throw' } catch (e) { e.constructor.name }").unwrap(),
        "TypeError"
    );
    // URLSearchParams: get/getAll/has/append/set/delete/toString/iteration.
    let p = "var p = new URLSearchParams('a=1&b=2&a=3');";
    assert_eq!(s.eval(&format!("{p} p.get('a')")).unwrap(), "1");
    assert_eq!(s.eval(&format!("{p} p.getAll('a').join(',')")).unwrap(), "1,3");
    assert_eq!(s.eval(&format!("{p} p.has('b') + ',' + p.has('z')")).unwrap(), "true,false");
    assert_eq!(s.eval(&format!("{p} p.append('c', '4'); p.toString()")).unwrap(), "a=1&b=2&a=3&c=4");
    assert_eq!(s.eval(&format!("{p} p.set('a', '9'); p.toString()")).unwrap(), "a=9&b=2");
    assert_eq!(s.eval(&format!("{p} p.delete('a'); p.toString()")).unwrap(), "b=2");
    // forEach and [Symbol.iterator] entries.
    assert_eq!(
        s.eval(&format!("{p} var o=[]; p.forEach((v,k)=>o.push(k+'='+v)); o.join('|')")).unwrap(),
        "a=1|b=2|a=3"
    );
    assert_eq!(
        s.eval(&format!("{p} Array.from(p).map(e=>e.join('=')).join('|')")).unwrap(),
        "a=1|b=2|a=3"
    );
    assert_eq!(s.eval(&format!("{p} Array.from(p.keys()).join(',')")).unwrap(), "a,b,a");
    assert_eq!(s.eval(&format!("{p} Array.from(p.values()).join(',')")).unwrap(), "1,2,3");
    // '+' decodes to a space; other inits (array, object, copy) parse too.
    assert_eq!(s.eval("new URLSearchParams('q=a+b').get('q')").unwrap(), "a b");
    assert_eq!(s.eval("new URLSearchParams([['k','v']]).get('k')").unwrap(), "v");
    assert_eq!(s.eval("new URLSearchParams({k:'v'}).get('k')").unwrap(), "v");
    assert_eq!(s.eval("new URLSearchParams(new URLSearchParams('k=v')).get('k')").unwrap(), "v");
    // searchParams edits reflect into the owning URL's search + href (one source).
    let url = "var url = new URL('https://e.com/p?x=1#h'); url.searchParams.set('y','2');";
    assert_eq!(s.eval(&format!("{url} url.search")).unwrap(), "?x=1&y=2");
    assert_eq!(s.eval(&format!("{url} url.href")).unwrap(), "https://e.com/p?x=1&y=2#h");
    // Deleting the last query pair empties search (href drops the '?').
    let url2 = "var u2 = new URL('https://e.com/p?x=1'); u2.searchParams.delete('x');";
    assert_eq!(s.eval(&format!("{url2} u2.search")).unwrap(), "");
    assert_eq!(s.eval(&format!("{url2} u2.href")).unwrap(), "https://e.com/p");
}

#[test]
fn refused_navigations_are_counted_no_ops() {
    let s = sess("https://example.com/", "ua/1");
    assert_eq!(s.denials(), 0);
    // href/whole-location assignment and the navigation methods all no-op, and
    // location keeps reading the real URL afterwards.
    s.eval("location.href = 'https://evil.test/'; location.assign('x');").unwrap();
    s.eval("location.replace('y'); location.reload();").unwrap();
    s.eval("window.location = 'z'; document.location = 'q';").unwrap();
    assert_eq!(s.eval("location.href").unwrap(), "https://example.com/");
    assert_eq!(s.denials(), 6);
    assert_eq!(s.eval("String(__frot_denied())").unwrap(), "undefined");
    assert_eq!(s.denials(), 7);
}

#[test]
fn history_is_in_memory_and_never_navigates() {
    let s = sess("https://example.com/", "ua/1");
    // Born fresh; a router probing `history.state` reads null, never throws.
    assert_eq!(s.eval("String(history.state)").unwrap(), "null");
    assert_eq!(s.eval("history.length").unwrap(), "1");
    assert_eq!(s.eval("history.scrollRestoration").unwrap(), "auto");
    // pushState sets `.state` and bumps length; replaceState only replaces.
    s.eval("history.pushState({page: 1}, '', '/a')").unwrap();
    assert_eq!(s.eval("history.state.page").unwrap(), "1");
    assert_eq!(s.eval("history.length").unwrap(), "2");
    s.eval("history.replaceState({page: 2}, '', '/b')").unwrap();
    assert_eq!(s.eval("history.state.page").unwrap(), "2");
    assert_eq!(s.eval("history.length").unwrap(), "2");
    // Navigation is refused: location stays the honest fetched URL, no denial.
    s.eval("history.go(-1); history.back(); history.forward();").unwrap();
    assert_eq!(s.eval("location.href").unwrap(), "https://example.com/");
    assert_eq!(s.denials(), 0);
}

#[test]
fn storage_and_cookie_are_in_memory_and_born_empty() {
    let s = sess("https://example.com/", "ua/1");
    assert_eq!(s.eval("localStorage.length").unwrap(), "0");
    assert_eq!(s.eval("document.cookie").unwrap(), "");
    s.eval("localStorage.setItem('a', '1'); localStorage.b = '2';").unwrap();
    assert_eq!(s.eval("localStorage.getItem('a')").unwrap(), "1");
    assert_eq!(s.eval("localStorage.b").unwrap(), "2");
    assert_eq!(s.eval("localStorage.length").unwrap(), "2");
    assert_eq!(s.eval("'a' in localStorage").unwrap(), "true");
    s.eval("localStorage.removeItem('a'); delete localStorage.b;").unwrap();
    assert_eq!(s.eval("localStorage.length").unwrap(), "0");
    assert_eq!(s.eval("String(sessionStorage.getItem('x'))").unwrap(), "null");
    // Cookie jar: set, read back, then expire away.
    s.eval("document.cookie = 'sid=42; path=/'; document.cookie = 'k=v';").unwrap();
    assert_eq!(s.eval("document.cookie").unwrap(), "sid=42; k=v");
    s.eval("document.cookie = 'sid=; max-age=0';").unwrap();
    assert_eq!(s.eval("document.cookie").unwrap(), "k=v");
}

#[test]
fn absent_apis_stay_undefined_and_window_aliases_the_global() {
    let s = sess("https://example.com/", "ua/1");
    assert_eq!(s.eval("window === globalThis && self === globalThis").unwrap(), "true");
    assert_eq!(s.eval("top === self && parent === self").unwrap(), "true");
    assert_eq!(s.eval("typeof indexedDB").unwrap(), "undefined");
    assert_eq!(s.eval("typeof Worker").unwrap(), "undefined");
    assert_eq!(s.eval("typeof WebSocket").unwrap(), "undefined");
    // Canvas getContext is a legal null; non-canvas has no context method.
    assert_eq!(s.eval("String(document.createElement('canvas').getContext('2d'))").unwrap(), "null");
    assert_eq!(
        s.eval("String(document.createElement('div').getContext('2d'))").unwrap(),
        "undefined"
    );
}
