//! A script's request to reload its own document (`challenge.md` §12): which
//! navigations are one, by the HTML "navigate" rule the `reloads` predicate
//! states, and that every one of them is still a counted denial.

use super::reloads;
use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, Session, StyleSource};

const PAGE: &str = "https://example.com/p?q=1";

fn asked(page: &str, script: &str) -> (bool, u32) {
    let s = Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        Env {
            url: page.into(),
            user_agent: "frot-test/1".into(),
            accept_language: "en-US,en;q=0.5".into(),
            images: false,
        },
        &FetchSession::new(Vec::new()),
    );
    s.eval(script).unwrap();
    (s.reload_asked(), s.denials())
}

#[test]
fn the_predicate_is_the_html_navigate_rule() {
    assert!(reloads(PAGE, ""));
    assert!(reloads(PAGE, PAGE));
    assert!(reloads(PAGE, "/p?q=1"));
    assert!(reloads("https://example.com", "https://example.com/"));
    // A document reached with a fragment reloads on `''` or its fragment-less
    // URL; the same URL *with* a fragment is a fragment navigation, not a load.
    assert!(reloads("https://example.com/#a", ""));
    assert!(!reloads("https://example.com/#a", "https://example.com/#a"));
    assert!(!reloads(PAGE, "#top"));
    assert!(!reloads(PAGE, "/p?q=2"));
    assert!(!reloads(PAGE, "https://other.example/p?q=1"));
    assert!(!reloads(PAGE, "http://[::1"));
    assert!(!reloads("not a url", ""));
}

#[test]
fn reload_replace_assign_and_href_of_the_own_url_ask_for_a_reload() {
    for script in [
        "location.reload()",
        "location.replace(location.href)",
        // Akamai's interstitial strips its own token parameter first.
        "window.location.replace(window.location.href.replace(/[&?]bm-verify=[^#]*/, ''))",
        "location.assign('/p?q=1')",
        "location.href = location.href",
        "location = String(location)",
        "document.location = location.href",
    ] {
        assert_eq!(asked(PAGE, script), (true, 1), "{script}");
    }
    // Asked once is asked: a later navigation elsewhere does not un-ask it.
    let twice = "location.reload(); location.assign('/elsewhere'); location.reload()";
    assert_eq!(asked(PAGE, twice), (true, 3));
}

#[test]
fn any_other_navigation_is_only_a_denial() {
    for script in [
        "location.assign('/elsewhere')",
        "location.replace(location.href + '#frag')",
        "location.href = 'https://other.example/'",
        "location.search = '?q=1'",
        "location.hash = 'x'",
    ] {
        let (reload, denials) = asked(PAGE, script);
        assert_eq!((reload, denials), (false, 1), "{script}");
    }
}
