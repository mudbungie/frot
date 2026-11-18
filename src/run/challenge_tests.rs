//! The declared-challenge / bot-defence verdict family: a server that itself
//! declares a success response is a stand-in for the page (`needs.md` §3 —
//! `retry-after`, `cf-mitigated: challenge`, `x-amzn-waf-action: challenge`)
//! is never reported as the page. Without `--js` the verdict is `needs:["js"]`
//! (a recipe change genuinely helps); with it the page's own script runs, and a
//! script that does not change the request ends in `needs:["human"]` — the
//! round trip that follows a script that *does* is pinned in
//! `roundtrip_tests.rs` (`docs/design/challenge.md`). A defence that refuses
//! with a status of 400 or above keeps `error{http.<code>}`, distinguishable from a genuine failure by
//! the surfaced markers. These are the offline pins
//! for the live corpus in `identity.md` §3.7/§3.9 — the field evidence is
//! dated and unrepeatable, the classification it established is not.

use super::*;

fn run_capture(args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let code = run_io(&argv, &mut out, &mut err);
    (
        code,
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

fn parse_envelope(s: &str) -> Value {
    serde_json::from_str(s.trim()).expect("envelope JSON")
}

#[test]
fn retry_after_on_200_without_js_needs_js() {
    // Field trial (bl-6d75): reddit's bot-challenge interstitial answers
    // HTTP 200 + `retry-after: 0` with a body that has a title and text — a
    // stand-in the server itself declared (needs.md §3). It must never be
    // reported as an `ok` impression of the page.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("Retry-After", "0")
        .with_body("<html><head><title>Please wait</title></head><body><main>verifying</main></body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0, "needs is not an error");
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    // challenge.md §5.2: `--js` is the recipe that can answer it, so the
    // honest signal names it — and says a challenge was declared, by what.
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(
        v["challenge"],
        serde_json::json!({"declared": "retry-after", "rounds": 0, "passed": false})
    );
    assert_eq!(v["http"]["status"], 200);
    assert!(v.get("out").is_none(), "no impression of a placeholder");
    assert!(v.get("error").is_none(), "nothing failed");
}

#[test]
fn cf_mitigated_challenge_without_js_needs_js() {
    // Cloudflare declares a challenge response with `cf-mitigated:
    // challenge` (needs.md §3); at a status < 400 that declaration — not the
    // body — flips the envelope.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("CF-Mitigated", "challenge")
        .with_body("<html><body>Checking your browser</body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(v["challenge"]["declared"], "cf-mitigated");
}

#[test]
fn amzn_waf_challenge_202_that_mints_nothing_needs_human() {
    // Field trial (bl-7e34, measured 2026-07-22 from the reference egress,
    // `docs/design/identity.md` §3):
    // www.amazon.com answers frot's own request headers with `202` +
    // `x-amzn-waf-action: challenge` + `server: CloudFront` and an AWS WAF
    // `challenge.js` body carrying no rendered text. Before this ball the
    // starvation detector (needs.md §4) read that text-free body as an SPA
    // shell and reported `needs:["js"]` — a lie: no recipe change renders a
    // page the origin refused to serve. The AWS WAF declaration is the same
    // transport fact as Cloudflare's, so it takes the same verdict. Under
    // `--js` the page's script is run (challenge.md §4.2); here `challenge.js`
    // does not load, nothing changes the request, and frot stops at one GET.
    let mut server = mockito::Server::new();
    let page = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_header("server", "CloudFront")
        .with_body("<html><body><script src=\"/challenge.js\"></script></body></html>")
        .expect(1)
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--js", "--out", "text"]);
    assert_eq!(code, 0, "needs is not an error");
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(
        v["needs"],
        serde_json::json!(["human"]),
        "a withheld body is not a page that needs JS"
    );
    assert_eq!(v["http"]["status"], 202);
    assert!(v.get("out").is_none(), "no impression of a placeholder");
    page.assert();
    assert_eq!(
        v["challenge"],
        serde_json::json!({"declared": "x-amzn-waf-action", "rounds": 0, "passed": false})
    );
    assert_eq!(v["js"]["scripts"], 0, "{v}");
    assert_eq!(v["js"]["errors"], 1, "the script that did not load: {v}");
    let hs = headers_of(&v);
    assert!(
        hs.contains(&("x-amzn-waf-action".into(), "challenge".into())),
        "the declaring header is surfaced: {hs:?}"
    );
}

#[test]
fn cloudfront_alone_is_not_a_refusal() {
    // The over-trigger guard (bl-7e34, and `bl-160d` case 4). This ball was
    // filed on the premise that `server: CloudFront` separates a soft block
    // from a genuine response; §3.9 falsified it. CloudFront fronts an
    // enormous amount of genuine content, so a CDN name must never by itself
    // flip the envelope — only the vendor *action* header declares. Same
    // status, same `server`, no declaration: an ordinary `ok`.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(202)
        .with_header("server", "CloudFront")
        .with_body("<html><body><p>real content</p></body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "ok", "a CDN name is not a bot-defence refusal");
    assert_eq!(v["out"], "real content");
}

fn headers_of(v: &Value) -> Vec<(String, String)> {
    v["http"]["headers"]
        .as_array()
        .expect("http.headers is always present on a network response")
        .iter()
        .map(|h| {
            (
                h["name"].as_str().unwrap().to_string(),
                h["value"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[test]
fn challenge_header_that_fired_needs_is_the_one_surfaced() {
    // One capture, two consumers (needs.md §3): the exact header that made
    // `needs::challenge` fire (`retry-after`) is the header `http.headers`
    // surfaces — proof they derive from the same capture and cannot disagree.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "0")
        .with_header("server", "snooserv")
        .with_body("<html><body><main>verifying</main></body></html>")
        .create();
    let (_c, out, _) = run_capture(&[&server.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["needs"], serde_json::json!(["js"]));
    assert_eq!(v["challenge"]["declared"], "retry-after");
    let hs = headers_of(&v);
    assert!(
        hs.contains(&("retry-after".into(), "0".into())),
        "the declaring header is surfaced: {hs:?}"
    );
}

#[test]
fn bot_defence_403_is_distinguishable_from_a_genuine_403() {
    // Field corpus (identity.md §3.7): g2's bot-defence 403 carried
    // `x-datadome: protected` + `server: cloudflare`; a genuine origin 403 does
    // not. Both flip to `error{http.403}`, but `http.headers` now carries the
    // vendor markers, so a caller tells a refusal from a real failure.
    let mut defence = mockito::Server::new();
    let _d = defence
        .mock("GET", "/")
        .with_status(403)
        .with_header("x-datadome", "protected")
        .with_header("server", "cloudflare")
        .with_body("<html>blocked</html>")
        .create();
    let (_c, out, _) = run_capture(&[&defence.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["error"]["kind"], "http.403");
    let hs = headers_of(&v);
    assert!(
        hs.contains(&("x-datadome".into(), "protected".into())),
        "vendor marker distinguishes the refusal: {hs:?}"
    );

    let mut genuine = mockito::Server::new();
    let _g = genuine
        .mock("GET", "/")
        .with_status(403)
        .with_header("server", "nginx")
        .with_body("<h1>Forbidden</h1>")
        .create();
    let (_c, out, _) = run_capture(&[&genuine.url(), "--out", "text"]);
    let v = parse_envelope(&out);
    assert_eq!(v["error"]["kind"], "http.403");
    assert!(
        headers_of(&v).iter().all(|(n, _)| n != "x-datadome"),
        "a genuine 403 carries no bot-defence marker"
    );
}

#[test]
fn a_declared_challenge_flips_every_view() {
    // The flip is view-independent, mirroring the >= 400 flip: a `--out dom
    // --js` of an interstitial whose script ran and minted nothing reports
    // `needs:["human"]`, with the `js` block of that run and no `out`.
    let mut server = mockito::Server::new();
    let _m = server
        .mock("GET", "/")
        .with_status(200)
        .with_header("retry-after", "5")
        .with_body("<html><body><script>window.solved=1;</script></body></html>")
        .create();
    let (code, out, _) = run_capture(&[&server.url(), "--js", "--out", "dom"]);
    assert_eq!(code, 0);
    let v = parse_envelope(&out);
    assert_eq!(v["status"], "needs");
    assert_eq!(v["needs"], serde_json::json!(["human"]));
    assert_eq!(v["js"]["scripts"], 1, "{v}");
    assert_eq!(v["js"]["settled"], true, "{v}");
    assert!(v.get("out").is_none(), "no impression of a placeholder");
}
