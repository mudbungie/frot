//! A request's [`Lineage`] across redirect hops, as Firefox 153.0esr sends it
//! (`bl-b298`, identity.md §3.22): the referrer never becomes a hop, a trimmed
//! referrer stays trimmed, and one cross-site hop taints the rest of the chain.

use super::*;

const PAGE: &str = "https://site.example/page";
const OTHER: &str = "https://other.test/hop";

fn value<'a>(h: &'a [(String, String)], name: &str) -> Option<&'a str> {
    h.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

/// The headers `lineage` puts on a request for `target`.
fn sent(intent: Intent, target: &str, lineage: &Lineage) -> Vec<(String, String)> {
    derive_headers(intent, target, target, lineage, &[], None, None)
}

#[test]
fn a_typed_navigation_sends_no_referrer_and_site_none_at_every_hop() {
    let mut l = Lineage::new(None);
    l.redirected(OTHER.to_string(), None);
    let h = sent(Intent::Navigation, PAGE, &l);
    assert_eq!(value(&h, "Referer"), None);
    assert_eq!(value(&h, "Sec-Fetch-Site"), Some("none"));
    assert_eq!(value(&h, "Sec-Fetch-User"), Some("?1"));
    // The redirect chain is still the cookie context.
    assert_eq!(l.context(), [OTHER]);
}

#[test]
fn a_followed_refresh_keeps_its_document_as_referrer_through_a_redirect() {
    let mut l = Lineage::new(Some(PAGE));
    let first = sent(Intent::Navigation, "https://site.example/r", &l);
    l.redirected(
        "https://site.example/r".into(),
        value(&first, "Referer").map(Into::into),
    );
    let h = sent(Intent::Navigation, "https://site.example/land", &l);
    assert_eq!(value(&h, "Referer"), Some(PAGE));
    assert_eq!(value(&h, "Sec-Fetch-Site"), Some("same-origin"));
    // Not user-activated: the document directed this navigation.
    assert_eq!(value(&h, "Sec-Fetch-User"), None);
}

#[test]
fn a_cross_site_bounce_taints_the_site_and_keeps_the_referrer_trimmed() {
    // page → site/x → other/hop → site/img.png: Firefox sends the origin only
    // and `cross-site` on the last hop, though it is the page's own origin.
    let mut l = Lineage::new(Some(PAGE));
    let x = "https://site.example/x";
    l.redirected(
        x.into(),
        value(&sent(Intent::Style, x, &l), "Referer").map(Into::into),
    );
    let hop = sent(Intent::Style, OTHER, &l);
    assert_eq!(value(&hop, "Sec-Fetch-Site"), Some("cross-site"));
    l.redirected(OTHER.into(), value(&hop, "Referer").map(Into::into));
    let back = sent(Intent::Style, "https://site.example/img.png", &l);
    assert_eq!(value(&back, "Referer"), Some("https://site.example/"));
    assert_eq!(value(&back, "Sec-Fetch-Site"), Some("cross-site"));
    // A same-site hop alone is `same-site`.
    let cdn = sent(
        Intent::Style,
        "https://cdn.site.example/a",
        &Lineage::new(Some(PAGE)),
    );
    assert_eq!(value(&cdn, "Sec-Fetch-Site"), Some("same-site"));
}
