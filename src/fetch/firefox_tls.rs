//! The persona-shaped `rustls` ClientConfig for HTTPS (Option C).
//!
//! WAFs fingerprint the TLS ClientHello (JA3/JA4). This module builds frot's
//! HTTPS client config with a coherent, *current, maintained* Firefox-leaning
//! shape using **stock `rustls 0.23` + `aws-lc-rs`** — no ClientHello-crafting
//! fork. That is Mark's Option-C ruling (2026-07-21, `docs/design/identity.md`
//! §6.1): retiring the abandoned `craftls` fork buys the security posture §6.3
//! calls the single most important fact (rustls CVEs now arrive by `cargo
//! update`), at the price of a byte-exact handshake. The persona facts frot
//! *can* enforce through stock rustls are wired from the one profile
//! ([`FIREFOX_153_ESR`]): the ALPN offer (`h2, http/1.1`), the key-exchange
//! group order (X25519MLKEM768 first), and the cipher order. What stock rustls
//! cannot reproduce — the full 17-cipher list, the Firefox extension order/set,
//! three key shares — is a **declared residual** in the §12 oracle, never a
//! silent miss.
//!
//! The config is consumed by [`super::transport`], which wraps it in a
//! hyper-rustls connector so one client speaks both h2 and http/1.1 by ALPN. No
//! rustls handshake type leaks past that seam.

use std::sync::Arc;

use rustls::crypto::aws_lc_rs::{cipher_suite, kx_group};
use rustls::crypto::{aws_lc_rs, CryptoProvider, SupportedKxGroup};
use rustls::{ClientConfig, RootCertStore, SupportedCipherSuite};

use super::profile::FIREFOX_153_ESR;

/// The Mozilla webpki root set (the roots ureq/rustls default to). The tests
/// inject a throwaway CA instead so a local server is reachable with real
/// verification.
pub(crate) fn webpki_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots
}

/// Key-exchange groups in the persona's order, mapped from the profile's IANA
/// ids to the aws-lc-rs groups. aws-lc-rs (rustls 0.23) offers X25519MLKEM768,
/// x25519, secp256r1 and secp384r1 — so those four map through in the persona's
/// order (PQ hybrid first). secp521r1 (25) and the two FFDHE groups (256, 257)
/// the persona also lists are not offered by aws-lc-rs, a declared residual
/// (§6.4), so they map to nothing.
fn firefox_kx_groups() -> Vec<&'static dyn SupportedKxGroup> {
    FIREFOX_153_ESR
        .tls
        .groups
        .iter()
        .filter_map(|&g| match g {
            4588 => Some(kx_group::X25519MLKEM768),
            29 => Some(kx_group::X25519),
            23 => Some(kx_group::SECP256R1),
            24 => Some(kx_group::SECP384R1),
            _ => None,
        })
        .collect()
}

/// The AEAD cipher suites rustls implements, in the persona's wire order. The
/// persona's legacy CBC/RSA suites (making its list 17 long) are not implemented
/// by rustls, so the advertised list is these nine — a declared residual on the
/// JA4 cipher component (Mark, 2026-07-21).
fn firefox_cipher_suites() -> Vec<SupportedCipherSuite> {
    vec![
        cipher_suite::TLS13_AES_128_GCM_SHA256,
        cipher_suite::TLS13_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS13_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
        cipher_suite::TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384,
        cipher_suite::TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384,
    ]
}

/// The persona client config over `roots`: a custom aws-lc-rs provider carrying
/// the persona's cipher and group order, TLS 1.2+1.3, real certificate
/// verification. ALPN is deliberately left unset here — `super::transport` hands
/// this config to hyper-rustls, which owns the ALPN offer (`h2, http/1.1`) and
/// rejects a pre-defined one; the persona's ALPN order (profile `alpn`) is the
/// same `h2, http/1.1`, and the handshake tests assert it end to end.
pub(crate) fn firefox_client_config(roots: RootCertStore) -> ClientConfig {
    let provider = CryptoProvider {
        cipher_suites: firefox_cipher_suites(),
        kx_groups: firefox_kx_groups(),
        ..aws_lc_rs::default_provider()
    };
    ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
        .expect("aws-lc-rs provider supports TLS 1.2 and 1.3")
        .with_root_certificates(roots)
        .with_no_client_auth()
}

#[cfg(test)]
mod tests;
