//! The h2 preface oracle (identity.md §12) — what frot actually puts on the wire
//! in its SETTINGS frame and the connection-level WINDOW_UPDATE that follows it.
//!
//! A throwaway-CA TLS origin negotiating `h2` by ALPN reads the client's raw
//! preface bytes instead of speaking HTTP, so the assertions below pin the exact
//! SETTINGS *set, order and values* — the akamai-h2 fingerprint's first two
//! fields. Every entry is either a profile fact frot enforces or a declared
//! residual named in the assertion, so neither can drift silently.

use std::io::Read;
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rustls::{RootCertStore, ServerConfig, ServerConnection, StreamOwned};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use super::Transport;
use crate::fetch::profile::FIREFOX_140_ESR;
use crate::fetch::MAX_BODY_BYTES;

/// The client connection preface (RFC 9113 §3.4), sent before the first frame.
const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

fn test_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            include_bytes!("../firefox_tls/testdata/ca.der").to_vec(),
        ))
        .unwrap();
    roots
}

fn server_config() -> Arc<ServerConfig> {
    let leaf = CertificateDer::from(include_bytes!("../firefox_tls/testdata/leaf.der").to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        include_bytes!("../firefox_tls/testdata/leaf.key.der").to_vec(),
    ));
    let mut cfg = ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(vec![leaf], key)
    .unwrap();
    cfg.alpn_protocols = vec![b"h2".to_vec()];
    Arc::new(cfg)
}

/// One decoded h2 frame header plus its payload.
struct Frame {
    kind: u8,
    stream: u32,
    payload: Vec<u8>,
}

/// Split `buf` (the preface followed by whole frames) into frames, stopping at
/// the first truncated one.
fn frames(buf: &[u8]) -> Vec<Frame> {
    let mut out = Vec::new();
    let mut p = PREFACE.len();
    while p + 9 <= buf.len() {
        let len = u32::from_be_bytes([0, buf[p], buf[p + 1], buf[p + 2]]) as usize;
        if p + 9 + len > buf.len() {
            break;
        }
        out.push(Frame {
            kind: buf[p + 3],
            stream: u32::from_be_bytes([buf[p + 5], buf[p + 6], buf[p + 7], buf[p + 8]]),
            payload: buf[p + 9..p + 9 + len].to_vec(),
        });
        p += 9 + len;
    }
    out
}

/// The `(identifier, value)` pairs of a SETTINGS payload, in wire order.
fn settings(payload: &[u8]) -> Vec<(u16, u32)> {
    payload
        .chunks(6)
        .map(|c| {
            (
                u16::from_be_bytes([c[0], c[1]]),
                u32::from_be_bytes([c[2], c[3], c[4], c[5]]),
            )
        })
        .collect()
}

/// A TLS origin that negotiates h2, reads the client preface and the frames that
/// follow it, then drops the connection. Returns its port and the captured bytes.
fn spawn_preface_recorder() -> (u16, mpsc::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (sock, _) = listener.accept().unwrap();
        let conn = ServerConnection::new(server_config()).unwrap();
        let mut tls = StreamOwned::new(conn, sock);
        // The preface, SETTINGS and WINDOW_UPDATE are written back-to-back after
        // the handshake; read until both frames are in hand (a short read only
        // means more is coming, so keep reading until the parse is satisfied).
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        while frames(&buf).len() < 2 {
            let n = tls.read(&mut chunk).unwrap();
            buf.extend_from_slice(&chunk[..n]);
        }
        let _ = tx.send(buf);
    });
    (port, rx)
}

#[test]
fn a_frame_split_across_reads_is_not_parsed_until_it_is_whole() {
    // The recorder reads until the parse yields two frames; that only terminates
    // because a partially-arrived frame is withheld rather than parsed short.
    let mut buf = PREFACE.to_vec();
    buf.extend_from_slice(&[0, 0, 6, 0x4, 0, 0, 0, 0, 0]); // SETTINGS, 6-byte body
    assert!(frames(&buf).is_empty());
    buf.extend_from_slice(&[0, 2, 0, 0, 0, 0]);
    assert_eq!(settings(&frames(&buf)[0].payload), vec![(2, 0)]);
}

#[test]
fn the_h2_preface_carries_the_pinned_settings_and_window_update() {
    let (port, rx) = spawn_preface_recorder();
    // The origin never answers the request, so the fetch fails; the preface it
    // sent first is the whole point, so the error is discarded.
    let _ = Transport::new(test_roots()).request_once(
        &format!("https://localhost:{port}/"),
        &[],
        MAX_BODY_BYTES,
        Duration::from_secs(5),
    );
    let captured = rx.recv().unwrap();
    assert_eq!(&captured[..PREFACE.len()], PREFACE);
    let frames = frames(&captured);

    // Frame 1: SETTINGS. The set, its order and its values, expectations derived
    // from the profile so a persona edit moves the test with it. Firefox 140esr
    // sends `1:65536; 2:0; 4:131072; 5:16384`; the two differences are the §12
    // declared residuals asserted below, and nothing else may appear.
    let h2 = FIREFOX_140_ESR.h2;
    assert_eq!(frames[0].kind, 0x4, "first frame is SETTINGS");
    assert_eq!(
        settings(&frames[0].payload),
        vec![
            (2, u32::from(h2.enable_push)),
            (4, h2.initial_window_size),
            (5, h2.max_frame_size),
            // Residual: hyper always sends MAX_HEADER_LIST_SIZE at its default
            // (its config types the value `u32`, not `Option<u32>`), and Firefox
            // sends no id 6 at all. Pinned against frot's own emission so a
            // `cargo update` that changes hyper's default fails here.
            (6, 16_384),
        ]
    );
    // Residual: HEADER_TABLE_SIZE (id 1) is absent although the persona declares
    // it, because `hyper-util`'s pooled client builder has no setter for it.
    assert!(
        !settings(&frames[0].payload).iter().any(|&(id, _)| id == 1),
        "id 1 unreachable, persona value {} is a declared residual",
        h2.header_table_size
    );

    // Frame 2: the connection-level WINDOW_UPDATE, on stream 0. The *increment*
    // is the akamai-h2 fingerprint's second field and matches Firefox exactly.
    assert_eq!(frames[1].kind, 0x8, "second frame is WINDOW_UPDATE");
    assert_eq!(frames[1].stream, 0, "connection-level, not per-stream");
    assert_eq!(
        u32::from_be_bytes(frames[1].payload[..4].try_into().unwrap()),
        h2.connection_window_increment
    );
}
