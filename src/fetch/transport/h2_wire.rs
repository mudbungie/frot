//! Shared plumbing for the two h2 oracles — the throwaway-CA `h2` origin, the
//! frame splitter, the SETTINGS and HPACK readers, and the akamai-h2 fingerprint
//! builder.
//!
//! Every parser here is exercised twice: once against real bytes by
//! `h2_preface.rs` / `h2_request.rs`, and once against synthetic bytes in this
//! file for the shapes a single capture cannot produce. The synthetic tests are
//! *parser* tests; the wire facts are asserted only in the two oracle modules.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

use rustls::{RootCertStore, ServerConfig, ServerConnection, StreamOwned};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use crate::fetch::profile::FIREFOX_140_ESR;

/// The client connection preface (RFC 9113 §3.4), sent before the first frame.
pub(super) const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// HEADERS flag `PRIORITY` (RFC 9113 §6.2) — the bit Firefox sets and `h2` does not.
pub(super) const PRIORITY_FLAG: u8 = 0x20;

pub(super) fn test_roots() -> RootCertStore {
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
pub(super) struct Frame {
    pub kind: u8,
    pub flags: u8,
    pub stream: u32,
    pub payload: Vec<u8>,
}

/// Split `buf` (the preface followed by whole frames) into frames, stopping at
/// the first truncated one.
pub(super) fn frames(buf: &[u8]) -> Vec<Frame> {
    let mut out = Vec::new();
    let mut p = PREFACE.len();
    while p + 9 <= buf.len() {
        let len = u32::from_be_bytes([0, buf[p], buf[p + 1], buf[p + 2]]) as usize;
        if p + 9 + len > buf.len() {
            break;
        }
        out.push(Frame {
            kind: buf[p + 3],
            flags: buf[p + 4],
            stream: u32::from_be_bytes([buf[p + 5], buf[p + 6], buf[p + 7], buf[p + 8]]),
            payload: buf[p + 9..p + 9 + len].to_vec(),
        });
        p += 9 + len;
    }
    out
}

/// The `(identifier, value)` pairs of a SETTINGS payload, in wire order.
pub(super) fn settings(payload: &[u8]) -> Vec<(u16, u32)> {
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

/// A TLS origin negotiating `h2` that reads the client's raw bytes instead of
/// speaking HTTP. `greeting` is written before reading (an h2 server SETTINGS
/// frame, or nothing), and reading stops the first time `done` accepts the
/// frames decoded so far — a short read only means more is coming.
pub(super) fn spawn_origin(
    greeting: &'static [u8],
    done: fn(&[Frame]) -> bool,
) -> (u16, mpsc::Receiver<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (sock, _) = listener.accept().unwrap();
        let conn = ServerConnection::new(server_config()).unwrap();
        let mut tls = StreamOwned::new(conn, sock);
        tls.write_all(greeting).unwrap();
        tls.flush().unwrap();
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        while !done(&frames(&buf)) {
            let n = tls.read(&mut chunk).unwrap();
            assert!(n > 0, "the client hung up before the capture was complete");
            buf.extend_from_slice(&chunk[..n]);
        }
        let _ = tx.send(buf);
    });
    (port, rx)
}

/// The akamai-h2 fingerprint: SETTINGS, the connection WINDOW_UPDATE increment,
/// the PRIORITY frames (`0` when there are none), and the pseudo-header order.
pub(super) fn akamai(
    settings: &[(u16, u32)],
    window_update: u32,
    priority: &str,
    pseudo: &str,
) -> String {
    let entries: Vec<String> = settings.iter().map(|(id, v)| format!("{id}:{v}")).collect();
    format!("{}|{window_update}|{priority}|{pseudo}", entries.join(";"))
}

/// The persona's akamai-h2 fingerprint, derived entirely from `FIREFOX_140_ESR.h2`
/// — a *declaration*, computed the same way as the capture so neither is stored.
pub(super) fn persona_akamai() -> String {
    let h2 = FIREFOX_140_ESR.h2;
    akamai(
        &[
            (1, h2.header_table_size),
            (2, u32::from(h2.enable_push)),
            (4, h2.initial_window_size),
            (5, h2.max_frame_size),
        ],
        h2.connection_window_increment,
        &format!("{}:0:0:{}", h2.initial_stream_id, h2.priority_weight),
        h2.pseudo_order,
    )
}

/// The pseudo-headers of an HPACK block, in wire order, as the akamai letters
/// `m`/`a`/`s`/`p`.
///
/// Only the leading pseudo-header region is decoded, and that needs no Huffman
/// decoder: every request pseudo-header an h2 client emits names the static
/// table by index — `:method`/`:scheme` fully indexed, `:authority`/`:path` as a
/// literal with an indexed name — so the *name* is the index. Decoding stops at
/// the first field that is not a pseudo-header, which is where the region ends
/// (RFC 9113 §8.3).
pub(super) fn pseudo_order(block: &[u8]) -> String {
    let mut out: Vec<char> = Vec::new();
    let mut p = 0;
    while p < block.len() {
        let b = block[p];
        // Indexed field (`1xxxxxxx`) carries its value too; a literal with an
        // indexed name (`01xxxxxx` / `0000xxxx` / `0001xxxx`) is followed by one
        // length-prefixed value string.
        let (index, indexed) = match b {
            _ if b & 0x80 != 0 => (b & 0x7f, true),
            _ if b & 0x40 != 0 => (b & 0x3f, false),
            _ => (b & 0x0f, false),
        };
        let Some(letter) = static_pseudo(index) else {
            break;
        };
        out.push(letter);
        p = if indexed {
            p + 1
        } else {
            skip_string(block, p + 1)
        };
    }
    out.iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// The akamai letter of an HPACK static-table index naming a pseudo-header
/// (RFC 7541 Appendix A), or `None` for anything else — including `:status`
/// (8–14), which is a response header and so ends a request's pseudo region.
fn static_pseudo(index: u8) -> Option<char> {
    match index {
        1 => Some('a'),
        2..=3 => Some('m'),
        4..=5 => Some('p'),
        6..=7 => Some('s'),
        _ => None,
    }
}

/// The index just past the length-prefixed string at `block[p..]`. The length is
/// HPACK's 7-bit prefixed integer (RFC 7541 §5.1): a prefix of 127 means the
/// remainder follows as continuation octets, seven bits each.
fn skip_string(block: &[u8], mut p: usize) -> usize {
    let mut len = (block[p] & 0x7f) as usize;
    p += 1;
    if len == 0x7f {
        let mut shift = 0;
        loop {
            len += ((block[p] & 0x7f) as usize) << shift;
            shift += 7;
            p += 1;
            if block[p - 1] & 0x80 == 0 {
                break;
            }
        }
    }
    p + len
}

#[test]
fn a_frame_split_across_reads_is_not_parsed_until_it_is_whole() {
    // The origin reads until its `done` predicate is satisfied; that only
    // terminates because a partially-arrived frame is withheld rather than
    // parsed short.
    let mut buf = PREFACE.to_vec();
    buf.extend_from_slice(&[0, 0, 6, 0x4, 0, 0, 0, 0, 0]); // SETTINGS, 6-byte body
    assert!(frames(&buf).is_empty());
    buf.extend_from_slice(&[0, 2, 0, 0, 0, 0]);
    assert_eq!(settings(&frames(&buf)[0].payload), vec![(2, 0)]);
}

#[test]
fn the_hpack_reader_walks_every_field_form_and_stops_at_the_first_real_header() {
    // One synthetic block covering the encodings a capture cannot show at once:
    // an indexed `:path` (0x85 = `/index.html`), a literal-never-indexed
    // `:scheme` (0x16), a literal-no-indexing `:authority` (0x01) whose value is
    // long enough to need HPACK's integer continuation, an indexed `:method`,
    // then a `:status` index — a response pseudo, so the region ends there.
    let mut block = vec![0x85, 0x16, 3, b'a', b'b', b'c', 0x01, 0x7f];
    block.extend_from_slice(&[0x81, 0x01]); // 127 + 1 + (1 << 7) = a 256-byte value
    block.extend(std::iter::repeat_n(0u8, 256));
    block.extend_from_slice(&[0x42, 2, b'h', b'i', 0x88]);
    assert_eq!(pseudo_order(&block), "p,s,a,m");
    // A block that is nothing but a regular header yields no pseudo region.
    assert_eq!(pseudo_order(&[0x00, 1, b'x', 1, b'y']), "");
}

#[test]
fn the_akamai_fingerprint_is_built_from_the_profile_and_nowhere_else() {
    // A drift guard on the *builder*, not on the wire: it proves the four
    // akamai fields are laid out as `SETTINGS|WINDOW_UPDATE|PRIORITY|PSEUDO`
    // and that every value comes from `FIREFOX_140_ESR.h2`. The wire comparison
    // is `h2_request.rs`.
    let h2 = FIREFOX_140_ESR.h2;
    assert_eq!(
        persona_akamai(),
        format!(
            "1:{};2:0;4:{};5:{}|{}|{}:0:0:{}|{}",
            h2.header_table_size,
            h2.initial_window_size,
            h2.max_frame_size,
            h2.connection_window_increment,
            h2.initial_stream_id,
            h2.priority_weight,
            h2.pseudo_order
        )
    );
}
