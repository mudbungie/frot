//! The envelope's `url` block (bl-2832): `requested` is the caller's spelling
//! byte-for-byte, `final` is the *resolved* URL — one canonical rule whether or
//! not a redirect happened. The canonical form itself (default ports, case,
//! IDNA, escapes) is pinned at its source in `fetch::tests`.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (code, String::from_utf8(out).unwrap())
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

/// The `url` block of a `--out meta` run of `url`.
fn url_block(url: &str) -> (String, String) {
    let (code, out) = run_capture(&[url, "--out", "meta"]);
    assert_eq!(code, 0, "{out}");
    let v = parse_envelope(&out);
    (
        v["url"]["requested"].as_str().expect("requested").into(),
        v["url"]["final"].as_str().expect("final").into(),
    )
}

#[test]
fn host_root_final_carries_the_path_the_caller_omitted() {
    // The bug: `frot https://example.com --out meta` reported
    // `final: https://example.com` while Chrome's `location.href` — and the
    // README's own example — is `https://example.com/`. `requested` keeps the
    // caller's spelling; `final` is the resolved fact downstream resolution
    // consumes.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<title>root</title>")
        .expect_at_least(1)
        .create();
    let bare = server.url();
    assert!(!bare.ends_with('/'), "mockito hands out a bare authority");
    let (requested, landed) = url_block(&bare);
    assert_eq!(requested, bare, "the caller's spelling, byte for byte");
    assert_eq!(landed, format!("{bare}/"));
    // The explicit slash is the same resource and reports the same `final`;
    // only `requested` differs, which is its whole job.
    let slashed = format!("{bare}/");
    let (requested, also_landed) = url_block(&slashed);
    assert_eq!(requested, slashed);
    assert_eq!(also_landed, landed);
}

#[test]
fn a_landing_spells_final_the_same_with_and_without_a_redirect() {
    // The coherence the bug broke: a redirect hop always serialized a parsed
    // `Url`, while a terminal-first fetch echoed the caller's raw argument, so
    // normalization depended on whether a 301 happened to be in the way.
    let mut server = mockito::Server::new();
    let _root = server
        .mock("GET", "/")
        .with_status(200)
        .with_body("<title>root</title>")
        .expect_at_least(1)
        .create();
    let _hop = server
        .mock("GET", "/go")
        .with_status(301)
        .with_header("location", "/")
        .create();
    let bare = server.url();
    let (_, direct) = url_block(&bare);
    let (_, redirected) = url_block(&format!("{bare}/go"));
    assert_eq!(direct, redirected, "one canonical rule, both paths");
}

#[test]
fn a_query_survives_the_canonical_form() {
    // Normalizing is not rewriting: the query is part of the resource, so it
    // rides through untouched while the omitted root path is filled in. (The
    // rest of the canonical rules — default ports, case, IDNA, escapes — are
    // pinned at their source in `fetch::tests`, which cannot be reached
    // end-to-end against a loopback server on a random port.)
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("<title>q</title>")
        .create();
    let bare = server.url();
    let (requested, landed) = url_block(&format!("{bare}?q=1&r=2"));
    assert_eq!(requested, format!("{bare}?q=1&r=2"));
    assert_eq!(landed, format!("{bare}/?q=1&r=2"));
}
