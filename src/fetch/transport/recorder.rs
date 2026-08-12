//! ClientHello recorder — the §12 oracle for the TLS fingerprint layer under
//! Option C.
//!
//! A raw TCP server captures the exact ClientHello frot puts on the wire (before
//! any handshake completes) and parses its ordered fields. The assertions pin the
//! facts frot *controls* from the profile — the cipher suite **wire order** and
//! the key-exchange **group order** (X25519MLKEM768 first) — so a regression in
//! either fails the build.
//!
//! It deliberately does **not** pin rustls's extension ordering: that is a
//! declared residual (§6.1/§11), and pinning it would couple the test to rustls
//! internals and break on the `cargo update` that Option C exists to keep cheap.
//! What it *does* pin about the extension list is order-free and identity-
//! bearing: the three fingerprint-load-bearing extensions are present, the two
//! Firefox-only ones (§4.1 `record_size_limit`, `compress_certificate`) are
//! asserted **absent as declared residuals**, and the absence of GREASE — a real
//! match with the persona, not a residual — is asserted on both sides.
//! (`bl-7523` corrected §6.1/§12, which claimed the order was pinned.)
//!
//! The JA4 fingerprint is computed here from both the capture and the profile
//! (`ja4.rs`), never stored, which is what §12's "hashes are assertions, not
//! fixtures" and §4.2's "nothing to regenerate" actually require.

use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::ja4::{is_grease, Ja4Hello};
use super::Transport;
use crate::fetch::firefox_tls::webpki_roots;
use crate::fetch::profile::FIREFOX_153_ESR;
use crate::fetch::MAX_BODY_BYTES;

/// Parsed ordered fields of a ClientHello — everything JA4 reads plus the
/// `supported_groups` list.
struct ClientHello {
    ciphers: Vec<u16>,
    extensions: Vec<u16>,
    groups: Vec<u16>,
    sig_algs: Vec<u16>,
    versions: Vec<u16>,
    alpn: String,
}

fn be16(b: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([b[i], b[i + 1]])
}

/// The `u16` list of `len` bytes starting at `at`.
fn u16s(b: &[u8], at: usize, len: usize) -> Vec<u16> {
    b[at..at + len]
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect()
}

/// Parse a ClientHello record. Input is always a well-formed handshake from
/// rustls, so offsets are read directly (an out-of-range slice would panic in
/// core, not leave an uncovered arm here).
fn parse(buf: &[u8]) -> ClientHello {
    let mut p = 5 + 4 + 2 + 32; // record header, handshake header, version, random
    p += 1 + buf[p] as usize; // session_id
    let cs_len = be16(buf, p) as usize;
    p += 2;
    let ciphers = u16s(buf, p, cs_len);
    p += cs_len;
    p += 1 + buf[p] as usize; // compression methods
    p += 2; // extensions length
    let mut hello = ClientHello {
        ciphers,
        extensions: Vec::new(),
        groups: Vec::new(),
        sig_algs: Vec::new(),
        versions: Vec::new(),
        alpn: String::new(),
    };
    while p + 4 <= buf.len() {
        let ty = be16(buf, p);
        let len = be16(buf, p + 2) as usize;
        hello.extensions.push(ty);
        // Each body carries its own list-length prefix: two bytes for the u16
        // lists, one for `supported_versions`, and ALPN's outer two then a
        // one-byte length per protocol (only the first is JA4-relevant).
        if ty == 0x000a {
            hello.groups = u16s(buf, p + 6, be16(buf, p + 4) as usize);
        }
        if ty == 0x000d {
            hello.sig_algs = u16s(buf, p + 6, be16(buf, p + 4) as usize);
        }
        if ty == 0x002b {
            hello.versions = u16s(buf, p + 5, buf[p + 4] as usize);
        }
        if ty == 0x0010 {
            let first = buf[p + 6] as usize;
            hello.alpn = String::from_utf8(buf[p + 7..p + 7 + first].to_vec()).unwrap();
        }
        p += 4 + len;
    }
    hello
}

/// A raw TCP server that records the first ClientHello record and returns its
/// port and a channel carrying the captured bytes.
fn spawn_recorder() -> (u16, mpsc::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        // rustls writes the ClientHello in one call, so on loopback it lands in
        // one read — the same single-segment assumption the persona-head recorder
        // relies on. The PQ key share fits well under the buffer.
        let mut buf = vec![0u8; 1 << 16];
        let n = sock.read(&mut buf).unwrap();
        buf.truncate(n);
        let _ = tx.send(buf);
    });
    (port, rx)
}

/// One real ClientHello, taken off the wire through the production path.
fn capture() -> ClientHello {
    let (port, rx) = spawn_recorder();
    // The handshake will not complete (the recorder is not a TLS server); we only
    // need the ClientHello frot sends, so the request error is discarded.
    let _ = Transport::new(webpki_roots()).request_once(
        &format!("https://localhost:{port}/"),
        &[],
        MAX_BODY_BYTES,
        Duration::from_secs(5),
    );
    parse(&rx.recv().unwrap())
}

/// The persona's §4.1 lists as a JA4 input — a *declaration*, not an observation.
fn persona_hello() -> Ja4Hello<'static> {
    let tls = FIREFOX_153_ESR.tls;
    Ja4Hello {
        version: 0x0304,
        sni: true,
        ciphers: tls.ciphers,
        extensions: tls.extensions,
        sig_algs: tls.sig_algs,
        alpn: FIREFOX_153_ESR.alpn[0],
    }
}

#[test]
fn the_client_hello_carries_the_pinned_cipher_and_group_order() {
    let hello = capture();
    // Cipher suites, in the persona wire order (TLS 1.3 trio first) — the JA4
    // cipher *order* frot controls. The list is stock rustls's nine AEAD suites
    // plus the renegotiation-info SCSV (0x00ff) it always appends, not Firefox's
    // 17: a declared residual (§6.1), so only what rustls implements is on wire.
    assert_eq!(
        hello.ciphers,
        vec![0x1301, 0x1303, 0x1302, 0xc02b, 0xc02f, 0xcca9, 0xcca8, 0xc02c, 0xc030, 0x00ff]
    );
    // supported_groups, PQ hybrid first (X25519MLKEM768=0x11ec), then x25519,
    // secp256r1, secp384r1 — the four aws-lc-rs reaches, in profile order.
    assert_eq!(hello.groups, vec![0x11ec, 0x001d, 0x0017, 0x0018]);
    // The fingerprint-load-bearing extensions are present (order is rustls's, a
    // declared residual, so it is not pinned): supported_groups, ALPN, key_share.
    for ext in [0x000a, 0x0010, 0x0033] {
        assert!(
            hello.extensions.contains(&ext),
            "missing extension {ext:#06x}"
        );
    }
}

#[test]
fn the_firefox_only_extensions_are_absent_and_neither_side_greases() {
    let hello = capture();
    let tls = FIREFOX_153_ESR.tls;
    // Residuals, asserted *as* residuals (§11, §6.1): rustls has no API for
    // either extension, so frot omits both although the persona declares them.
    // The persona values are named in the message so the gap stays legible.
    assert!(
        !hello.extensions.contains(&28),
        "record_size_limit absent; persona value {} is a declared residual",
        tls.record_size_limit
    );
    assert!(
        !hello.extensions.contains(&27),
        "compress_certificate absent; persona algorithms {:?} are a declared residual",
        tls.cert_compression
    );
    // Absence of GREASE is a real MATCH, not a residual: Firefox 153esr sends
    // none and rustls sends none. Asserted on both the wire and the declaration
    // by the *class* — `ja4::is_grease`, the one definition of RFC 8701's
    // `0x?a?a` — so no GREASE literal is written down here at all.
    let greased = |v: &u16| is_grease(*v);
    assert!(!hello.extensions.iter().any(greased), "GREASE on the wire");
    assert!(
        !hello.ciphers.iter().any(greased),
        "GREASE cipher on the wire"
    );
    assert!(
        !tls.extensions.iter().any(greased) && !tls.ciphers.iter().any(greased),
        "the persona declares no GREASE either"
    );
}

#[test]
fn ja4_is_computed_from_both_sides_and_differs_exactly_where_option_c_says() {
    let captured = capture();
    let wire = Ja4Hello {
        version: *captured.versions.iter().max().unwrap(),
        sni: captured.extensions.contains(&0x0000),
        ciphers: &captured.ciphers,
        extensions: &captured.extensions,
        sig_algs: &captured.sig_algs,
        alpn: &captured.alpn,
    };
    let persona = persona_hello();
    // Neither string is stored: both are computed by the same function from a
    // single source (the wire, and §4.1). §6.1 states plainly that "JA4 does not
    // match the pin" — so the oracle asserts the mismatch rather than hiding it.
    assert_ne!(wire.ja4(), persona.ja4(), "§6.1 declares JA4 as a residual");
    // What Option C *does* reproduce is the JA4_a prefix's non-count half:
    // TCP + TLS 1.3 + SNI present, and `h2` as the first ALPN.
    let (w, p) = (wire.raw(true), persona.raw(true));
    assert_eq!(&w[..4], &p[..4], "protocol, version and SNI presence match");
    assert_eq!(&w[8..10], &p[8..10], "first ALPN is h2 on both sides");
    // The residual halves: the cipher list (9 AEAD + SCSV vs 17) and the
    // extension set both differ, so the two hashed components differ too.
    assert_ne!(
        &w[4..8],
        &p[4..8],
        "cipher/extension counts are the residual"
    );
    let (wf, pf) = (w.split('_'), p.split('_'));
    assert!(
        wf.zip(pf).skip(1).all(|(a, b)| a != b),
        "both JA4_r halves are residuals: {w} vs {p}"
    );
    // JA4_ro is the same fingerprint unsorted; it must stay a distinct reading of
    // the same capture, which is the only thing that makes the sorted form a
    // fingerprint rather than a transcript.
    assert_ne!(wire.raw(false), w);
}
