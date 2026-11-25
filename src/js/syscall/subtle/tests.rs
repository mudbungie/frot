//! Known-answer tests for the WebCrypto primitives — published vectors, never a
//! value the implementation produced for itself — and the syscalls that carry
//! them to the prelude (AES's vectors are in `aes::tests`). The `crypto.subtle` surface a page sees (shapes,
//! errors, key handling) is pinned in `js::tests::subtle`.

use super::alg;
use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, Session, StyleSource};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// FIPS 180-4's one-block message `"abc"`, and the empty message.
#[test]
fn digests_match_the_fips_180_examples() {
    let cases = [
        ("SHA-1", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        (
            "SHA-256",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "SHA-384",
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7",
        ),
        (
            "SHA-512",
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        ),
    ];
    for (name, want) in cases {
        assert_eq!(hex(&alg::digest(name, b"abc").unwrap()), want, "{name}");
    }
    assert_eq!(
        hex(&alg::digest("SHA-256", b"").unwrap()),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hex(&alg::digest("SHA-1", b"").unwrap()),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    // Not in the table: no digest, and no HMAC over it either.
    assert_eq!(alg::digest("MD5", b"abc"), None);
    assert_eq!(alg::hmac("SHA-224", b"k", b"abc"), None);
}

/// RFC 2202 (SHA-1) and RFC 4231 (SHA-2) test case 1: a 20-byte `0x0b` key over
/// `"Hi There"`; and test case 2, the short key `"Jefe"`.
#[test]
fn hmac_matches_rfc_2202_and_rfc_4231() {
    let key = [0x0b; 20];
    let cases = [
        ("SHA-1", "b617318655057264e28bc0b6fb378c8ef146be00"),
        (
            "SHA-256",
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        ),
        (
            "SHA-384",
            "afd03944d84895626b0825f4ab46907f15f9dadbe4101ec682aa034c7cebc59c\
             faea9ea9076ede7f4af152e8b2fa9cb6",
        ),
        (
            "SHA-512",
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde\
             daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
        ),
    ];
    for (name, want) in cases {
        assert_eq!(
            hex(&alg::hmac(name, &key, b"Hi There").unwrap()),
            want,
            "{name}"
        );
    }
    assert_eq!(
        hex(&alg::hmac("SHA-256", b"Jefe", b"what do ya want for nothing?").unwrap()),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
}

/// RFC 6070's PBKDF2-HMAC-SHA1 cases 1, 2 and 3 (`"password"`, `"salt"`, 1, 2
/// and 4096 iterations) and RFC 7914 §11's first PBKDF2-HMAC-SHA256 case.
#[test]
fn pbkdf2_matches_rfc_6070_and_rfc_7914() {
    let cases = [
        (1, "0c60c80f961f0e71f3a9b524af6012062fe037a6"),
        (2, "ea6c014dc72d6f8ccd1ed92ace1d41f0d8de8957"),
        (4096, "4b007901b765489abead49d926f721d065a429c1"),
    ];
    for (rounds, want) in cases {
        let out = alg::pbkdf2("SHA-1", b"password", b"salt", rounds, 20).unwrap();
        assert_eq!(hex(&out), want, "{rounds} iterations");
    }
    assert_eq!(
        hex(&alg::pbkdf2("SHA-256", b"passwd", b"salt", 1, 64).unwrap()),
        "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
         49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
    );
    // SHA-384 and SHA-512 are wired to their own HMACs: each output differs.
    let wide = ["SHA-1", "SHA-256", "SHA-384", "SHA-512"]
        .map(|name| alg::pbkdf2(name, b"password", b"salt", 1, 16).unwrap());
    assert!((0..4).all(|i| (0..i).all(|j| wide[i] != wide[j])));
    // No bytes asked for is no bytes; an empty password and salt are allowed.
    assert_eq!(alg::pbkdf2("SHA-1", b"", b"", 1, 0).unwrap(), b"");
    assert_eq!(alg::pbkdf2("SHA-1", b"", b"", 1, 4).unwrap().len(), 4);
}

/// The refusals: an unknown hash, no iterations, and more work than
/// [`alg::PBKDF2_WORK`] — which counts every output block.
#[test]
fn pbkdf2_refuses_what_it_cannot_do_and_what_would_not_end() {
    assert_eq!(alg::pbkdf2("MD5", b"p", b"s", 1, 20), None);
    assert_eq!(alg::pbkdf2("SHA-1", b"p", b"s", 0, 20), None);
    let most = u32::try_from(alg::PBKDF2_WORK).unwrap();
    assert_eq!(alg::pbkdf2("SHA-256", b"p", b"s", most + 1, 32), None);
    assert_eq!(alg::pbkdf2("SHA-256", b"p", b"s", most / 2 + 1, 33), None);
    assert_eq!(
        alg::pbkdf2("SHA-256", b"p", b"s", u32::MAX, usize::MAX),
        None
    );
    assert!(alg::pbkdf2("SHA-256", b"p", b"s", 1000, 33).is_some());
}

/// RFC 5869's test cases 1 (SHA-256), 3 (SHA-256, empty salt and info) and 4
/// (SHA-1).
#[test]
fn hkdf_matches_rfc_5869() {
    let salt = unhex("000102030405060708090a0b0c");
    let info = unhex("f0f1f2f3f4f5f6f7f8f9");
    assert_eq!(
        hex(&alg::hkdf("SHA-256", &[0x0b; 22], &salt, &info, 42).unwrap()),
        "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf\
         34007208d5b887185865"
    );
    assert_eq!(
        hex(&alg::hkdf("SHA-256", &[0x0b; 22], b"", b"", 42).unwrap()),
        "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d\
         9d201395faa4b61a96c8"
    );
    assert_eq!(
        hex(&alg::hkdf("SHA-1", &[0x0b; 11], &salt, &info, 42).unwrap()),
        "085a01ea1b10f36933068b56efa5ad81a4f14b822f5b091568a9cdd4f155fda2\
         c22e422478d305f3f896"
    );
    // The RFC's ceiling is 255 hash lengths; no bytes asked for is no bytes.
    assert_eq!(
        alg::hkdf("SHA-256", b"k", b"", b"", 255 * 32)
            .unwrap()
            .len(),
        8160
    );
    assert_eq!(alg::hkdf("SHA-256", b"k", b"", b"", 255 * 32 + 1), None);
    assert_eq!(alg::hkdf("SHA-384", b"k", b"", b"", 255 * 48 + 1), None);
    assert_eq!(alg::hkdf("SHA-512", b"", b"", b"", 0).unwrap(), b"");
    assert_eq!(alg::hkdf("MD5", b"k", b"", b"", 8), None);
}

fn sess() -> Session {
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
        images: false,
    };
    let s = Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    );
    s.eval(
        "function hex(u){return Array.from(u,function(b){return (b+256).toString(16).slice(1)})\
         .join('')} function U(n){return new Uint8Array(n)}",
    )
    .unwrap();
    s
}

/// The syscalls hand the prelude a fresh `Uint8Array` sized to the result, or
/// `undefined` for what the table does not compute.
#[test]
fn the_syscalls_carry_bytes_both_ways() {
    let s = sess();
    assert_eq!(
        s.eval("hex(__frot_subtle_digest('SHA-1', new Uint8Array([97,98,99])))")
            .unwrap(),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
    assert_eq!(
        s.eval(
            "(function(){var d=__frot_subtle_digest('SHA-256',U(0));\
             return [d instanceof Uint8Array,d.byteOffset,d.buffer.byteLength].join()})()"
        )
        .unwrap(),
        "true,0,32"
    );
    assert_eq!(
        s.eval("String(__frot_subtle_digest('MD5', U(1)))").unwrap(),
        "undefined"
    );
    assert_eq!(
        s.eval("hex(__frot_subtle_hmac('SHA-256', U(32), U(0)))")
            .unwrap(),
        "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
    );
    assert_eq!(
        s.eval("String(__frot_subtle_hmac('MD5', U(32), U(0)))")
            .unwrap(),
        "undefined"
    );
    assert_eq!(
        s.eval("hex(__frot_subtle_gcm(false, U(32), U(12), U(0), 16, U(16)))")
            .unwrap(),
        "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919"
    );
    // Each cipher round-trips through its two directions; `open` on what was
    // never sealed is `undefined`.
    let msg = "new Uint8Array([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16])";
    for (seal, open, junk) in [
        (
            "__frot_subtle_gcm(false, U(32), U(12), U(0), 12, M)",
            "__frot_subtle_gcm(true, U(32), U(12), U(0), 12, C)",
            "__frot_subtle_gcm(true, U(32), U(12), U(0), 12, U(20))",
        ),
        (
            "__frot_subtle_cbc(false, U(24), U(16), M)",
            "__frot_subtle_cbc(true, U(24), U(16), C)",
            "__frot_subtle_cbc(true, U(24), U(16), U(16))",
        ),
        (
            "__frot_subtle_ctr(U(16), U(16), 64, M)",
            "__frot_subtle_ctr(U(16), U(16), 64, C)",
            "__frot_subtle_ctr(U(16), U(16), 0, U(16))",
        ),
        (
            "__frot_subtle_kw(false, U(16), M)",
            "__frot_subtle_kw(true, U(16), C)",
            "__frot_subtle_kw(true, U(16), U(24))",
        ),
    ] {
        let run = format!("(function(){{var M={msg},C={seal};return hex({open})+' '+{junk}}})()");
        assert_eq!(
            s.eval(&run).unwrap(),
            "0102030405060708090a0b0c0d0e0f10 undefined",
            "{seal}"
        );
    }
    let abc = "new Uint8Array([97,98,99])";
    assert_eq!(
        s.eval(&format!(
            "hex(__frot_subtle_pbkdf2('SHA-1', {abc}, {abc}, 2, 4))"
        ))
        .unwrap()
        .len(),
        8
    );
    assert_eq!(
        s.eval("String(__frot_subtle_pbkdf2('SHA-1', U(1), U(1), 0, 4))")
            .unwrap(),
        "undefined"
    );
    assert_eq!(
        s.eval(&format!(
            "hex(__frot_subtle_hkdf('SHA-256', {abc}, U(0), U(0), 5))"
        ))
        .unwrap()
        .len(),
        10
    );
    assert_eq!(
        s.eval("String(__frot_subtle_hkdf('MD5', U(1), U(0), U(0), 5))")
            .unwrap(),
        "undefined"
    );
    // A detached view has no bytes: it reads as the empty message.
    assert_eq!(
        s.eval(
            "(function(){var u=new Uint8Array([1,2,3]);u.buffer.transfer();\
             return hex(__frot_subtle_digest('SHA-1',u))})()"
        )
        .unwrap(),
        "da39a3ee5e6b4b0d3255bfef95601890afd80709"
    );
    // Not bytes at all: the binding refuses, it does not guess.
    assert!(s.eval("__frot_subtle_digest('SHA-1', 'abc')").is_err());
}
