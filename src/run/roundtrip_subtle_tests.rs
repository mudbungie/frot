//! The round trip through a challenge shaped like the one measured on AWS WAF
//! (`docs/design/challenge.md` §11.3–§11.4): the script fetches its inputs,
//! imports the page's 32-byte key with `crypto.subtle.importKey("raw", …,
//! {name: "AES-GCM"}, …, ["encrypt"])`, encrypts its answer, and POSTs it. The
//! server only mints the token for the **correct** ciphertext — a published
//! vector, so a `SubtleCrypto` that answered anything else would fail here.

use super::*;
use mockito::Matcher::{self, Missing};

fn run_capture(args: &[&str]) -> Value {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    run_io(&argv, &mut out, &mut err);
    serde_json::from_str(String::from_utf8(out).unwrap().trim()).expect("envelope JSON")
}

/// The challenge page: key and IV in `window.gokuProps`, as measured, then the
/// script under test.
fn challenge(server: &mut mockito::Server, script: &str) -> mockito::Mock {
    let props = format!("{{key:'{}',iv:'{}'}}", "00".repeat(32), "00".repeat(12));
    server
        .mock("GET", "/")
        .match_header("cookie", Missing)
        .with_status(202)
        .with_header("x-amzn-waf-action", "challenge")
        .with_body(format!(
            "<script>window.gokuProps={props};\
             function unhex(s){{return Uint8Array.from(s.match(/../g),function(h){{\
             return parseInt(h,16)}})}}\
             function hex(b){{return Array.from(new Uint8Array(b),function(x){{\
             return (x+256).toString(16).slice(1)}}).join('')}}\
             {script}</script>"
        ))
        .expect(1)
        .create()
}

const SOLVE: &str = "fetch('/inputs?client=browser').then(function(r){return r.json()})\
    .then(function(inputs){\
      return crypto.subtle.importKey('raw',unhex(gokuProps.key),{name:'AES-GCM'},false,\
        ['encrypt']).then(function(key){\
        return Promise.all([\
          crypto.subtle.encrypt({name:'AES-GCM',iv:unhex(gokuProps.iv)},key,\
            unhex(inputs.challenge)),\
          crypto.subtle.digest('SHA-256',new Uint8Array([97,98,99]))])})})\
    .then(function(r){\
      return fetch('/verify',{method:'POST',body:JSON.stringify({solution:hex(r[0]),\
        checksum:hex(r[1])})})});";

#[test]
fn a_challenge_that_encrypts_its_answer_with_subtle_crypto_is_passed() {
    let mut server = mockito::Server::new();
    let gate = challenge(&mut server, SOLVE);
    let inputs = server
        .mock("GET", "/inputs?client=browser")
        .with_body(format!("{{\"challenge\":\"{}\"}}", "00".repeat(16)))
        .expect(1)
        .create();
    // AES-256-GCM of one zero block under a zero key and nonce (the GCM
    // specification's test case 14), and FIPS 180-4's SHA-256("abc").
    let verify = server
        .mock("POST", "/verify")
        .match_body(Matcher::Exact(
            "{\"solution\":\"cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919\",\
             \"checksum\":\"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\"}"
                .into(),
        ))
        .with_header("set-cookie", "aws-waf-token=minted; Path=/")
        .with_body("{}")
        .expect(1)
        .create();
    let real = server
        .mock("GET", "/")
        .match_header("cookie", "aws-waf-token=minted")
        .with_body("<title>Shop</title><p>the real page</p>")
        .expect(1)
        .create();
    let v = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    inputs.assert();
    verify.assert();
    real.assert();
    assert_eq!(v["status"], "ok", "{v}");
    assert_eq!(v["out"], "the real page");
    assert_eq!(
        v["challenge"],
        serde_json::json!({"declared": "x-amzn-waf-action", "rounds": 1, "passed": true})
    );
}

/// The body shape measured on AWS WAF (`challenge.md` §11.4): the answer goes
/// out as a `FormData`, so the POST is `multipart/form-data` text.
#[test]
fn a_challenge_that_posts_its_answer_as_form_data_is_passed() {
    let mut server = mockito::Server::new();
    let gate = challenge(
        &mut server,
        "crypto.subtle.digest('SHA-256',new Uint8Array([97,98,99])).then(function(d){\
           var fd=new FormData(); fd.append('solution_data',JSON.stringify({checksum:hex(d)}));\
           fd.append('signals','c2lnbmFscw==');\
           return fetch('/telemetry',{body:fd,method:'POST'})});",
    );
    let telemetry = server
        .mock("POST", "/telemetry")
        .match_header(
            "content-type",
            Matcher::Regex("^multipart/form-data; boundary=----geckoformboundary[0-9a-f]+$".into()),
        )
        .match_body(Matcher::Regex(
            "(?s)name=\"solution_data\"\r\n\r\n\\{\"checksum\":\"ba7816bf8f01cfea414140de5dae2223\
             b00361a396177a9cb410ff61f20015ad\"\\}\r\n--.*name=\"signals\"\r\n\r\nc2lnbmFscw==\r\n"
                .into(),
        ))
        .with_header("set-cookie", "aws-waf-token=minted; Path=/")
        .with_body("{}")
        .expect(1)
        .create();
    let real = server
        .mock("GET", "/")
        .match_header("cookie", "aws-waf-token=minted")
        .with_body("<p>the real page</p>")
        .expect(1)
        .create();
    let v = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    telemetry.assert();
    real.assert();
    assert_eq!(v["out"], "the real page", "{v}");
    assert_eq!(v["challenge"]["passed"], true);
}

/// A challenge that needs an algorithm frot has not implemented gets a
/// rejection it can see — not a fabricated key — so nothing is POSTed, no
/// round is taken, and the stop is the honest one.
#[test]
fn a_challenge_needing_an_unimplemented_algorithm_stops_honestly() {
    let mut server = mockito::Server::new();
    let gate = challenge(
        &mut server,
        "crypto.subtle.importKey('raw',unhex(gokuProps.key),'Ed25519',false,['verify'])\
         .then(function(){return fetch('/verify',{method:'POST',body:'never'})});",
    );
    let verify = server.mock("POST", "/verify").expect(0).create();
    let v = run_capture(&[&server.url(), "--js", "--out", "text"]);
    gate.assert();
    verify.assert();
    assert_eq!(v["needs"], serde_json::json!(["human"]), "{v}");
    assert_eq!(
        v["challenge"],
        serde_json::json!({"declared": "x-amzn-waf-action", "rounds": 0, "passed": false})
    );
    assert_eq!(v["js"]["errors"], 1, "{v}");
}
