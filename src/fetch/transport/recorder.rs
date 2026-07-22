//! ClientHello recorder — the §12 oracle for the TLS fingerprint layer, scoped
//! to what `bl-abca` ships under Option C.
//!
//! A raw TCP server captures the exact ClientHello frot puts on the wire (before
//! any handshake completes) and parses its ordered fields. The assertions pin the
//! facts frot *controls* from the profile — the cipher suite **wire order** and
//! the key-exchange **group order** (X25519MLKEM768 first) — so a regression in
//! either fails the build. It deliberately does **not** pin rustls's extension
//! ordering: that is a declared residual (§6.1/§11), and pinning it would couple
//! the test to rustls internals and break on the `cargo update` that Option C
//! exists to keep cheap. The full four-layer golden is `bl-d66b`'s (§12).

use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::Transport;
use crate::fetch::firefox_tls::webpki_roots;
use crate::fetch::MAX_BODY_BYTES;

/// Parsed ordered fields of a ClientHello: cipher suites, extension types (in
/// wire order), and the `supported_groups` list.
struct ClientHello {
    ciphers: Vec<u16>,
    extensions: Vec<u16>,
    groups: Vec<u16>,
}

fn be16(b: &[u8], i: usize) -> u16 {
    u16::from_be_bytes([b[i], b[i + 1]])
}

/// Parse a ClientHello record. Input is always a well-formed handshake from
/// rustls, so offsets are read directly (an out-of-range slice would panic in
/// core, not leave an uncovered arm here).
fn parse(buf: &[u8]) -> ClientHello {
    let mut p = 5 + 4 + 2 + 32; // record header, handshake header, version, random
    p += 1 + buf[p] as usize; // session_id
    let cs_len = be16(buf, p) as usize;
    p += 2;
    let ciphers = buf[p..p + cs_len]
        .chunks(2)
        .map(|c| u16::from_be_bytes([c[0], c[1]]))
        .collect();
    p += cs_len;
    p += 1 + buf[p] as usize; // compression methods
    p += 2; // extensions length
    let mut extensions = Vec::new();
    let mut groups = Vec::new();
    while p + 4 <= buf.len() {
        let ty = be16(buf, p);
        let len = be16(buf, p + 2) as usize;
        extensions.push(ty);
        if ty == 0x000a {
            let list_len = be16(buf, p + 4) as usize;
            groups = buf[p + 6..p + 6 + list_len]
                .chunks(2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();
        }
        p += 4 + len;
    }
    ClientHello {
        ciphers,
        extensions,
        groups,
    }
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

#[test]
fn the_client_hello_carries_the_pinned_cipher_and_group_order() {
    let (port, rx) = spawn_recorder();
    // The handshake will not complete (the recorder is not a TLS server); we only
    // need the ClientHello frot sends, so the request error is discarded.
    let _ = Transport::new(webpki_roots()).request_once(
        &format!("https://localhost:{port}/"),
        &[],
        MAX_BODY_BYTES,
        Duration::from_secs(5),
    );
    let hello = parse(&rx.recv().unwrap());

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
