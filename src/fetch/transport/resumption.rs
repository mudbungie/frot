//! The TLS-resumption oracle (identity.md §6.5 reason 3, §12) — a second
//! connection to an origin already visited *inside one invocation* takes the
//! abbreviated handshake, not a fresh one.
//!
//! §6.5 accepts the h1 re-dial residual (§11) partly because the extra socket
//! resumes the TLS session exactly as a browser's second connection does. That
//! rests on two facts a `cargo update` could flip silently: `firefox_client_config`
//! does not override `ClientConfig::resumption`, and rustls 0.23's default is
//! `Resumption::in_memory_sessions(256)`. One `ClientConfig` is built per
//! [`Transport`] — i.e. per invocation, the object a `FetchSession` owns — and
//! shared by every connection it dials, so the ticket stored on the first
//! handshake is offered on the second. This module measures that instead of
//! reading it: the origin reports rustls's own `handshake_kind()` per accepted
//! connection, and the pin is `Full` then `Resumed`.
//!
//! **Deterministic by construction, not by race.** The origin negotiates
//! `http/1.1` by ALPN and answers with `Connection: close`, so hyper retires the
//! socket instead of pooling it and the next same-origin request *must* dial
//! afresh — nothing here depends on the eventual pool check-in that §11 declares
//! and that flaked `bl-df88`. The requests are sequential and each body is fully
//! read before the next begins, so the client has processed the server's
//! NewSessionTicket by the time it offers one.
//!
//! The session-level analogue is unreachable offline: `FetchSession` builds its
//! transport over the webpki roots, which cannot verify the throwaway CA, so the
//! pin is taken one layer down at the object that actually owns the shared
//! `ClientConfig`.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use rustls::{HandshakeKind, RootCertStore, ServerConfig, ServerConnection, StreamOwned};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use super::Transport;
use crate::fetch::MAX_BODY_BYTES;

/// The response every accepted connection answers with. `Connection: close` is
/// the whole trick: it forbids hyper from pooling the socket, so request two
/// dials a second connection deterministically.
const RESPONSE: &[u8] =
    b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";

fn test_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            include_bytes!("../firefox_tls/testdata/ca.der").to_vec(),
        ))
        .unwrap();
    roots
}

/// The throwaway-CA origin config, ALPN `http/1.1` only. Session storage and
/// ticket issuance are rustls's defaults — the server half of the claim under
/// test is that a stock origin's offer is *taken*, not that frot configures one.
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
    cfg.alpn_protocols = vec![b"http/1.1".to_vec()];
    Arc::new(cfg)
}

/// Serve exactly one request on `tls` (read the head, answer, hang up) and
/// report the handshake rustls performed for that connection.
fn serve_one(
    tls: &mut StreamOwned<ServerConnection, std::net::TcpStream>,
) -> Option<HandshakeKind> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        tls.read_exact(&mut byte).unwrap();
        head.extend_from_slice(&byte);
    }
    tls.write_all(RESPONSE).unwrap();
    tls.flush().unwrap();
    tls.conn.handshake_kind()
}

/// An h1 TLS origin that accepts exactly `n` connections in order, answering one
/// request on each and closing. Returns its port and the per-connection
/// handshake kinds, in accept order.
fn spawn_h1_origin(n: usize) -> (u16, mpsc::Receiver<Option<HandshakeKind>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let cfg = server_config();
        for _ in 0..n {
            let (sock, _) = listener.accept().unwrap();
            let conn = ServerConnection::new(cfg.clone()).unwrap();
            let mut tls = StreamOwned::new(conn, sock);
            let kind = serve_one(&mut tls);
            tx.send(kind).unwrap();
        }
    });
    (port, rx)
}

#[test]
fn a_second_connection_in_one_invocation_resumes_the_tls_session() {
    let (port, kinds) = spawn_h1_origin(2);
    // One Transport = one invocation = one `ClientConfig`, shared by both dials.
    let transport = Transport::new(test_roots());
    for path in ["/first", "/second"] {
        let resp = transport
            .request_once(
                &format!("https://localhost:{port}{path}"),
                &[],
                MAX_BODY_BYTES,
                Duration::from_secs(15),
            )
            .unwrap();
        // Reading the body to completion is what guarantees the first exchange
        // is finished — and its NewSessionTicket processed — before the second
        // request offers a ticket.
        assert_eq!(resp.body, b"ok");
    }
    let first = kinds.recv().unwrap();
    let second = kinds.recv().unwrap();
    assert_eq!(
        first,
        Some(HandshakeKind::Full),
        "cold origin, full handshake"
    );
    assert_eq!(
        second,
        Some(HandshakeKind::Resumed),
        "the re-dial must offer the stored ticket: identity.md §6.5 reason 3 \
         rests on rustls's default `Resumption::in_memory_sessions(256)` and on \
         `firefox_client_config` not overriding it"
    );
}
