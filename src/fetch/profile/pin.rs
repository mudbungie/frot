//! The one pinned persona — the §4.1 table, and nothing else.
//!
//! Split out of `profile.rs` so §4.2's claim is literally true of the tree: a
//! re-pin is an edit to **this file**, and the shapes, derivations and oracles
//! that read it live elsewhere. Nothing here is computed; everything here is a
//! measured fact of Mozilla Firefox 140.12.0esr.

use super::{BrowserProfile, CivilDate, H2Profile, TlsProfile};

/// The pinned persona: Mozilla Firefox 140.12.0esr, linux-x86_64 en-US
/// (identity.md §2). Captured 2026-07-19 via Marionette from Gecko's own
/// necko/NSS stack.
pub const FIREFOX_140_ESR: BrowserProfile = BrowserProfile {
    name: "Firefox",
    version: "140.12.0esr",
    major: 140,
    product_sub: "20100101",
    platform: "Linux x86_64",
    language: "en-US",
    build_id: "20181001000000",
    hardware_concurrency: 8,
    color_depth: 24,
    canvas_seed: 0x_f00d_c0de, // opaque fixed seed; mixed with draws, never raw.
    timer_precision_us: 1_000, // 1 ms — Firefox's reduceTimerPrecision default
    // ESR 140 is the sole current ESR line (`FIREFOX_ESR_NEXT` empty, §2); when
    // its successor lands this trips and forces a re-capture + re-pin.
    eol: CivilDate {
        year: 2027,
        month: 6,
        day: 1,
    },
    alpn: &["h2", "http/1.1"],
    tls: TlsProfile {
        // 4588=X25519MLKEM768, 29=x25519, 23=secp256r1, 24=secp384r1,
        // 25=secp521r1, 256=ffdhe2048, 257=ffdhe3072.
        groups: &[4588, 29, 23, 24, 25, 256, 257],
        key_share_groups: &[4588, 29, 23],
        // ecdsa_secp256r1_sha256, ecdsa_secp384r1_sha384, ecdsa_secp521r1_sha512,
        // rsa_pss_rsae_sha256/384/512, rsa_pkcs1_sha256/384/512,
        // ecdsa_sha1, rsa_pkcs1_sha1.
        sig_algs: &[
            0x0403, 0x0503, 0x0603, 0x0804, 0x0805, 0x0806, 0x0401, 0x0501, 0x0601, 0x0203, 0x0201,
        ],
        // 17 suites, wire order — the TLS 1.3 trio, then ECDHE AEAD, then the
        // legacy CBC/RSA tail (§4.1; §12 pins `0xc009` at index 10).
        ciphers: &[
            0x1301, 0x1303, 0x1302, 0xc02b, 0xc02f, 0xcca9, 0xcca8, 0xc02c, 0xc030, 0xc00a, 0xc009,
            0xc013, 0xc014, 0x009c, 0x009d, 0x002f, 0x0035,
        ],
        // 17 extension types, wire order, ECH (65037) last (§4.1).
        extensions: &[
            0, 23, 65281, 10, 11, 35, 16, 5, 34, 18, 51, 43, 13, 45, 28, 27, 65037,
        ],
        record_size_limit: 16_385,
        cert_compression: &[1, 2, 3], // zlib, brotli, zstd
    },
    h2: H2Profile {
        header_table_size: 65_536,
        enable_push: false,
        initial_window_size: 131_072,
        max_frame_size: 16_384,
        connection_window_increment: 12_517_377,
        initial_stream_id: 3,
        pseudo_order: "m,p,a,s",
        priority_weight: 42,
    },
    accept_document: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    accept_style: "text/css,*/*;q=0.1",
    accept_default: "*/*",
};
