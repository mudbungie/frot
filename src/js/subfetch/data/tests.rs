//! `data:` URL decoding tests (js.md §4.1/§6): the RFC 2397 shapes a real page
//! uses — base64 and percent-encoded payloads, media-type parameters, the RFC
//! default type, malformed encodings — and the §6 cache behaviour that follows
//! from a resource carrying its own bytes.

use super::*;

fn body(url: &str) -> String {
    decode(url, payload(url).unwrap()).unwrap().body
}

#[test]
fn a_plain_payload_is_percent_decoded() {
    // The URL parser escapes what it must; decoding is what puts the source
    // back. `%2C` is a comma that is *not* the separator.
    assert_eq!(
        body("data:text/javascript,a%20=%20%22x%2Cy%22"),
        "a = \"x,y\""
    );
}

#[test]
fn a_base64_payload_is_decoded_with_whitespace_and_padding_ignored() {
    // "let a = 1;" — split across lines, as an HTML attribute may be.
    assert_eq!(
        body("data:text/javascript;base64,bGV0\nIGEg\t PSAxOw=="),
        "let a = 1;"
    );
    // The whole alphabet decodes, `+` and `/` included: `Pj4+Pz8/` is `>>>???`,
    // the two digits a minified bundle hits within its first few hundred bytes.
    assert_eq!(body("data:text/javascript;base64,Pj4+Pz8/"), ">>>???");
}

#[test]
fn the_base64_suffix_is_matched_case_insensitively_past_parameters() {
    assert_eq!(
        body("data:application/x-javascript;charset=utf-8;BASE64,eA=="),
        "x"
    );
}

#[test]
fn a_charset_parameter_decodes_the_bytes_like_a_content_type_header() {
    // 0xE9 is é in latin-1 and invalid UTF-8: the declared charset decides.
    assert_eq!(body("data:text/javascript;charset=iso-8859-1,%E9"), "é");
    // The type also rides out as the response's Content-Type.
    let url = "data:text/javascript;charset=iso-8859-1,%E9";
    let f = decode(url, payload(url).unwrap()).unwrap();
    assert_eq!(
        f.headers,
        vec![(
            "content-type".into(),
            "text/javascript;charset=iso-8859-1".into()
        )]
    );
    assert!(f.ok);
    assert_eq!(f.status, 200);
    assert_eq!(f.url, url);
}

#[test]
fn an_absent_media_type_falls_back_to_the_rfc_default() {
    let url = "data:,hello";
    let f = decode(url, payload(url).unwrap()).unwrap();
    assert_eq!(f.body, "hello");
    assert_eq!(f.headers[0].1, DEFAULT_MIME);
    // ";base64" alone is a flag with no type, and still gets the default.
    let url = "data:;base64,aGk=";
    let f = decode(url, payload(url).unwrap()).unwrap();
    assert_eq!(f.body, "hi");
    assert_eq!(f.headers[0].1, DEFAULT_MIME);
}

#[test]
fn only_a_data_url_has_a_payload() {
    assert_eq!(payload("data:,x"), Some(",x"));
    assert_eq!(payload("https://example.com/a.js"), None);
}

#[test]
fn a_payload_without_a_comma_is_malformed() {
    let e = decode("data:text/javascript", "text/javascript").unwrap_err();
    assert!(e.contains("no comma"), "{e}");
}

#[test]
fn a_bad_base64_payload_is_malformed_and_named_briefly() {
    // `*` is outside the alphabet.
    let url = "data:text/javascript;base64,****";
    let e = decode(url, payload(url).unwrap()).unwrap_err();
    assert!(e.contains("malformed base64"), "{e}");
    // A lone trailing sextet carries no whole byte.
    let url = "data:text/javascript;base64,eAAAA";
    assert!(decode(url, payload(url).unwrap()).is_err());
    // A long payload is not copied whole into the message.
    let url = format!("data:text/javascript;base64,{}*", "A".repeat(400));
    let e = decode(&url, payload(&url).unwrap()).unwrap_err();
    assert!(e.len() < 120, "unbounded message: {} bytes", e.len());
}

#[test]
fn brief_keeps_a_short_url_whole() {
    assert_eq!(brief("data:,x"), "data:,x");
}

mod through_the_cache {
    use std::time::Duration;

    use crate::fetch::{FetchSession, Intent};
    use crate::js::engine::{Clock, NetBudget};
    use crate::js::subfetch::{Outcome, Subfetch};

    const HI: &str = "data:text/javascript,globalThis.x%20=%201";

    /// A cache whose *network* budget is already spent — the state in which
    /// every real dispatch is refused (js.md §6).
    fn spent_budget(budget: usize) -> Subfetch {
        let net = NetBudget::on(Clock::wall(), Duration::ZERO);
        Subfetch::with_budget(
            FetchSession::new(Vec::new()),
            "https://example.com/",
            net,
            budget,
        )
    }

    fn get(sf: &mut Subfetch, spec: &str) -> Outcome {
        sf.get(spec, Intent::ClassicScript)
    }

    #[test]
    fn a_data_url_is_served_past_the_network_budget() {
        // Nothing is dispatched, so the spent budget that refuses network has
        // nothing to refuse — and the run is not marked `stopped: network` on
        // account of an inline script.
        let mut sf = spent_budget(64);
        match get(&mut sf, HI) {
            Outcome::Got(f) => assert_eq!(f.body, "globalThis.x = 1"),
            Outcome::Failed(m) => panic!("data URL refused: {m}"),
        }
        assert!(!sf.refused());
        assert!(
            sf.timings().is_empty(),
            "a decode is not a measured request"
        );
    }

    #[test]
    fn decoded_bytes_charge_the_pooled_byte_budget() {
        // The one bound that still applies: a pool of 16 bytes takes the
        // 16-byte body, and the next data URL is refused by the same message
        // a fetched body would earn.
        let mut sf = spent_budget(16);
        assert!(matches!(get(&mut sf, HI), Outcome::Got(_)));
        match get(&mut sf, "data:,more") {
            Outcome::Failed(m) => assert!(m.contains("byte budget exhausted"), "{m}"),
            Outcome::Got(_) => panic!("the byte pool was ignored"),
        }
    }

    #[test]
    fn warm_leaves_data_urls_alone() {
        // The preload scanner hides latency; a data URL has none, so it is
        // not warmed — and is still served when the queue reaches it.
        let mut sf = spent_budget(64);
        sf.warm(&[(HI.to_string(), Intent::ClassicScript)]);
        assert!(!sf.refused());
        assert!(matches!(get(&mut sf, HI), Outcome::Got(_)));
    }
}
