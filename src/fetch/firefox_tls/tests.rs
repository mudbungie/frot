//! The persona rustls config: kx-group order (with the residual groups
//! filtered) and cipher order. ALPN and the handshake itself — ALPN-selected h2
//! and h1 fallback against local TLS servers with verification on — are
//! exercised end to end in `transport/tests.rs`.

use super::*;

#[test]
fn kx_groups_are_the_four_reachable_persona_groups_pq_first() {
    // The profile lists seven; aws-lc-rs reaches four (secp521r1 + the two FFDHE
    // groups are declared residuals). One call exercises every match arm because
    // each listed group hits exactly one.
    let groups = firefox_kx_groups();
    assert_eq!(groups.len(), 4);
    assert_eq!(u16::from(groups[0].name()), 4588); // X25519MLKEM768 first.
    assert_eq!(u16::from(groups[1].name()), 29); // x25519.
}

#[test]
fn cipher_suites_are_the_nine_aead_suites_tls13_first() {
    let suites = firefox_cipher_suites();
    assert_eq!(suites.len(), 9);
    assert_eq!(
        suites[0].suite(),
        rustls::CipherSuite::TLS13_AES_128_GCM_SHA256
    );
}

#[test]
fn client_config_leaves_alpn_to_hyper_rustls() {
    // hyper-rustls owns the ALPN offer and rejects a pre-defined one, so the
    // config must ship with none set.
    let config = firefox_client_config(webpki_roots());
    assert!(config.alpn_protocols.is_empty());
}
