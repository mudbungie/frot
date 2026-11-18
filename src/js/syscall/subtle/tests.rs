//! Known-answer tests for the WebCrypto primitives — published vectors, never a
//! value the implementation produced for itself — and the three syscalls that
//! carry them to the prelude. The `crypto.subtle` surface a page sees (shapes,
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

/// The McGrew–Viega GCM specification's test cases (the vectors NIST adopted):
/// 2 (AES-128), 8 (AES-192) and 14 (AES-256) encrypt one zero block under a zero
/// key and nonce; 4 carries associated data and a 60-byte plaintext.
#[test]
fn aes_gcm_matches_the_gcm_specification_vectors() {
    let zero_block = [0u8; 16];
    let cases = [
        (
            16,
            "0388dace60b6a392f328c2b971b2fe78ab6e47d42cec13bdf53a67b21257bddf",
        ),
        (
            24,
            "98e7247c07f0fe411c267e4384b0f6002ff58d80033927ab8ef4d4587514f0fb",
        ),
        (
            32,
            "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919",
        ),
    ];
    for (len, want) in cases {
        let key = vec![0u8; len];
        let sealed = alg::gcm_seal(&key, &[0; 12], &[], &zero_block).unwrap();
        assert_eq!(hex(&sealed), want, "{len}-byte key");
        assert_eq!(
            alg::gcm_open(&key, &[0; 12], &[], &sealed).unwrap(),
            zero_block
        );
    }

    let key = unhex("feffe9928665731c6d6a8f9467308308");
    let iv = unhex("cafebabefacedbaddecaf888");
    let aad = unhex("feedfacedeadbeeffeedfacedeadbeefabaddad2");
    let plain = unhex(
        "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72\
         1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39",
    );
    let sealed = alg::gcm_seal(&key, &iv, &aad, &plain).unwrap();
    assert_eq!(
        hex(&sealed),
        "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e\
         21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091\
         5bc94fbc3221a5db94fae95ae7121a47"
    );
    assert_eq!(alg::gcm_open(&key, &iv, &aad, &sealed).unwrap(), plain);
}

#[test]
fn aes_gcm_refuses_what_it_cannot_do_and_what_does_not_authenticate() {
    let key = [0u8; 32];
    let sealed = alg::gcm_seal(&key, &[0; 12], b"aad", b"secret").unwrap();
    // A flipped bit anywhere, other associated data, another key or nonce.
    let mut flipped = sealed.clone();
    flipped[0] ^= 1;
    assert_eq!(alg::gcm_open(&key, &[0; 12], b"aad", &flipped), None);
    assert_eq!(alg::gcm_open(&key, &[0; 12], b"", &sealed), None);
    assert_eq!(alg::gcm_open(&[1; 32], &[0; 12], b"aad", &sealed), None);
    assert_eq!(alg::gcm_open(&key, &[1; 12], b"aad", &sealed), None);
    // Shorter than a tag: nothing to authenticate.
    assert_eq!(alg::gcm_open(&key, &[0; 12], b"aad", &sealed[..4]), None);
    // A key that is not 16/24/32 bytes; a nonce that is not 12.
    assert_eq!(alg::gcm_seal(&[0; 5], &[0; 12], b"", b"x"), None);
    assert_eq!(alg::gcm_open(&[0; 5], &[0; 12], b"", &sealed), None);
    assert_eq!(alg::gcm_seal(&key, &[0; 16], b"", b"x"), None);
    assert_eq!(alg::gcm_open(&key, &[0; 16], b"", &sealed), None);
    // The empty message seals to a bare tag and opens to nothing.
    let tag = alg::gcm_seal(&key, &[0; 12], b"", b"").unwrap();
    assert_eq!(tag.len(), 16);
    assert_eq!(alg::gcm_open(&key, &[0; 12], b"", &tag).unwrap(), b"");
}

fn sess() -> Session {
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
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
        s.eval("hex(__frot_subtle_gcm(false, U(32), U(12), U(0), U(16)))")
            .unwrap(),
        "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919"
    );
    assert_eq!(
        s.eval(
            "hex(__frot_subtle_gcm(true, U(32), U(12), U(0), \
             __frot_subtle_gcm(false, U(32), U(12), U(0), new Uint8Array([1,2,3]))))"
        )
        .unwrap(),
        "010203"
    );
    assert_eq!(
        s.eval("String(__frot_subtle_gcm(true, U(32), U(12), U(0), U(20)))")
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
