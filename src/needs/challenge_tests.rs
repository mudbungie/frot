//! Transport-declared deferral (needs.md §3): which response headers declare a
//! challenge, and at which statuses.

use super::*;

fn hdr(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
}

#[test]
fn retry_after_declares_a_challenge_whatever_its_value() {
    assert_eq!(
        challenge(Some(200), &hdr(&[("retry-after", "0")])),
        Some("retry-after")
    );
    assert_eq!(
        challenge(Some(200), &hdr(&[("Retry-After", "120")])),
        Some("retry-after")
    );
}

#[test]
fn cf_mitigated_challenge_declares_a_challenge_case_insensitively() {
    assert_eq!(
        challenge(Some(200), &hdr(&[("cf-mitigated", "challenge")])),
        Some("cf-mitigated")
    );
    assert_eq!(
        challenge(Some(200), &hdr(&[("CF-Mitigated", " Challenge ")])),
        Some("cf-mitigated")
    );
}

#[test]
fn other_cf_mitigated_values_are_not_a_challenge() {
    // Cloudflare uses the header for other mitigations; only the challenge
    // declaration means "the page was never served".
    assert_eq!(
        challenge(Some(200), &hdr(&[("cf-mitigated", "block")])),
        None
    );
}

#[test]
fn amzn_waf_challenge_declares_a_challenge_case_insensitively() {
    // Measured 2026-07-22 (bl-7e34): www.amazon.com answers frot's exact
    // request headers with 202 + `x-amzn-waf-action: challenge` + an AWS WAF
    // `challenge.js` body. The declaration, not the empty-looking body, is
    // the fact.
    assert_eq!(
        challenge(Some(200), &hdr(&[("x-amzn-waf-action", "challenge")])),
        Some("x-amzn-waf-action")
    );
    assert_eq!(
        challenge(Some(200), &hdr(&[("X-Amzn-Waf-Action", " Challenge ")])),
        Some("x-amzn-waf-action")
    );
}

#[test]
fn other_amzn_waf_actions_are_not_a_challenge() {
    // AWS WAF also spells `captcha` / `block` here; only `challenge` is the
    // declared deferral this detector recognizes. (`captcha` at a 2xx would
    // fall through — it is not a deferral the server says to retry, and
    // inventing a verdict for an unmeasured shape is guessing.)
    assert_eq!(
        challenge(Some(200), &hdr(&[("x-amzn-waf-action", "block")])),
        None
    );
}

#[test]
fn ordinary_response_headers_declare_nothing() {
    assert_eq!(
        challenge(Some(200), &hdr(&[("content-type", "text/html")])),
        None
    );
    assert_eq!(challenge(None, &[]), None);
}

#[test]
fn every_declaring_header_is_surfaced_in_the_http_block() {
    // `challenge.declared` names a header the caller can also read in
    // `http.headers` (challenge.md §5.1): one capture, one allowlist.
    for name in DECLARING {
        assert!(crate::envelope::SURFACED.contains(&name), "{name}");
    }
}

#[test]
fn a_vendor_declaration_holds_at_any_status() {
    // `bl-a7c5`: Cloudflare serves its managed challenge at 403. The vendor's
    // own marker is the declaration; the status beside it is not consulted.
    for status in [Some(403), Some(503), None] {
        assert_eq!(
            challenge(status, &hdr(&[("cf-mitigated", "challenge")])),
            Some("cf-mitigated")
        );
        assert_eq!(
            challenge(status, &hdr(&[("x-amzn-waf-action", "challenge")])),
            Some("x-amzn-waf-action")
        );
        // Another action stays undeclared there too.
        assert_eq!(challenge(status, &hdr(&[("cf-mitigated", "block")])), None);
    }
}

#[test]
fn retry_after_declares_only_on_a_success_response() {
    // On a 429 or a 503 it is the header's ordinary meaning — a rate limit, an
    // outage — and the response is the error it says it is.
    let h = hdr(&[("retry-after", "30")]);
    assert_eq!(challenge(Some(399), &h), Some("retry-after"));
    assert_eq!(challenge(None, &h), Some("retry-after"));
    for refused in [400, 429, 503] {
        assert_eq!(challenge(Some(refused), &h), None);
    }
    // A vendor marker beside it still declares, and is the one named.
    let both = hdr(&[("retry-after", "30"), ("cf-mitigated", "challenge")]);
    assert_eq!(challenge(Some(503), &both), Some("cf-mitigated"));
    assert_eq!(challenge(Some(200), &both), Some("retry-after"));
}
