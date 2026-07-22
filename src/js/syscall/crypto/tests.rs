//! The crypto randomness syscall and the `crypto.js` prelude it backs, driven
//! through raw `__frot_*` calls and the `crypto` object, for 100% Rust coverage
//! without the JS-invisible golden suite. Randomness is real, so tests assert
//! type/length/error behaviour and *distinctness across calls* — never a pinned
//! byte value (identity.md §8: separate invocations share no identity state).

use crate::dom::Document;
use crate::fetch::FetchSession;
use crate::js::{Env, Session, StyleSource};

fn sess() -> Session {
    let env = Env {
        url: "https://example.com/".into(),
        user_agent: "frot-test/1".into(),
        accept_language: "en-US,en;q=0.5".into(),
    };
    Session::new(
        Document::parse("<html><body></body></html>"),
        StyleSource::Bare,
        env,
        &FetchSession::new(Vec::new()),
    )
}

#[test]
fn random_bytes_syscall_returns_the_requested_length() {
    let s = sess();
    assert_eq!(s.eval("__frot_random_bytes(0).length").unwrap(), "0");
    assert_eq!(s.eval("__frot_random_bytes(16).length").unwrap(), "16");
    // Every byte is a value in [0, 255] — a real OS byte, not undefined padding.
    assert_eq!(
        s.eval("__frot_random_bytes(32).every(b => b >= 0 && b <= 255)")
            .unwrap(),
        "true"
    );
}

#[test]
fn get_random_values_fills_and_returns_the_view() {
    let s = sess();
    // Returns the same view, of the same length, populated in place.
    assert_eq!(
        s.eval("crypto.getRandomValues(new Uint8Array(8)).length")
            .unwrap(),
        "8"
    );
    assert_eq!(
        s.eval("(function(){var a=new Uint8Array(4);return crypto.getRandomValues(a)===a;})()")
            .unwrap(),
        "true"
    );
    // A wider integer view is filled byte-wise across its whole byteLength.
    assert_eq!(
        s.eval("crypto.getRandomValues(new Uint32Array(4)).length")
            .unwrap(),
        "4"
    );
    // Separate invocations share no state: two 32-byte draws differ (the chance
    // of collision is 2^-256 — deterministic enough for a test).
    assert_eq!(
        s.eval(
            "(function(){var a=new Uint8Array(32),b=new Uint8Array(32);\
             crypto.getRandomValues(a);crypto.getRandomValues(b);\
             return a.join()!==b.join();})()"
        )
        .unwrap(),
        "true"
    );
}

#[test]
fn get_random_values_enforces_the_browser_error_contract() {
    let s = sess();
    // A non-integer typed view (Float64Array/DataView) is a TypeMismatchError.
    assert_eq!(
        s.eval(
            "(function(){try{crypto.getRandomValues(new Float64Array(2));return 'no';}\
             catch(e){return e.name;}})()"
        )
        .unwrap(),
        "TypeMismatchError"
    );
    assert_eq!(
        s.eval(
            "(function(){try{crypto.getRandomValues(new DataView(new ArrayBuffer(4)));\
             return 'no';}catch(e){return e.name;}})()"
        )
        .unwrap(),
        "TypeMismatchError"
    );
    // Over-quota (> 65536 bytes) is a QuotaExceededError.
    assert_eq!(
        s.eval(
            "(function(){try{crypto.getRandomValues(new Uint8Array(65537));return 'no';}\
             catch(e){return e.name;}})()"
        )
        .unwrap(),
        "QuotaExceededError"
    );
    // The 65536 boundary is allowed (returns the view).
    assert_eq!(
        s.eval("crypto.getRandomValues(new Uint8Array(65536)).length")
            .unwrap(),
        "65536"
    );
}

#[test]
fn random_uuid_is_a_v4_uuid() {
    let s = sess();
    // Shape: 8-4-4-4-12 hex, version nibble 4, variant nibble in [89ab].
    assert_eq!(
        s.eval(
            "/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/\
             .test(crypto.randomUUID())"
        )
        .unwrap(),
        "true"
    );
    // Two UUIDs differ — no shared identity state across calls.
    assert_eq!(
        s.eval("crypto.randomUUID() !== crypto.randomUUID()")
            .unwrap(),
        "true"
    );
}
