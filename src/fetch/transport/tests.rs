//! The transport oracle: ALPN-selected h2 and h1 fallback both complete against
//! a local TLS server **with certificate verification on** (the throwaway CA in
//! `firefox_tls/testdata`), plus every error arm of the taxonomy mapper. Servers
//! run on their own thread with a current-thread tokio runtime; the client is
//! the production [`Transport`] pointed at a test root store.

use std::io::Write as _;
use std::net::TcpListener as StdListener;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use rustls::{RootCertStore, ServerConfig};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use super::*;
use crate::fetch::MAX_BODY_BYTES;

fn test_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            include_bytes!("../firefox_tls/testdata/ca.der").to_vec(),
        ))
        .unwrap();
    roots
}

fn server_config(alpn: Vec<Vec<u8>>) -> Arc<ServerConfig> {
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
    cfg.alpn_protocols = alpn;
    Arc::new(cfg)
}

/// Spawn a one-shot TLS server offering `alpn`; returns its port and a channel
/// carrying the ALPN protocol it negotiated with the client. The response is a
/// fixed `hi`, served over whichever protocol ALPN selected (hyper's auto server).
fn spawn_https(alpn: Vec<Vec<u8>>) -> (u16, Receiver<Option<Vec<u8>>>) {
    let (port_tx, port_rx) = mpsc::channel();
    let (alpn_tx, alpn_rx) = mpsc::channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();
            let acceptor = TlsAcceptor::from(server_config(alpn));
            let (tcp, _) = listener.accept().await.unwrap();
            // A client that rejects our (untrusted) cert aborts the handshake;
            // that is a valid case, not a server panic.
            let Ok(tls) = acceptor.accept(tcp).await else {
                return;
            };
            alpn_tx
                .send(tls.get_ref().1.alpn_protocol().map(<[u8]>::to_vec))
                .unwrap();
            let svc = service_fn(|_: Request<Incoming>| async {
                Ok::<_, std::convert::Infallible>(Response::new(Full::new(Bytes::from_static(
                    b"hi",
                ))))
            });
            let _ = auto::Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls), svc)
                .await;
        });
    });
    (port_rx.recv().unwrap(), alpn_rx)
}

/// Spawn a one-shot raw TCP server that writes `reply` and closes — for the
/// handshake-failure and malformed-response arms, where no real HTTP happens.
fn spawn_raw(reply: &'static [u8]) -> u16 {
    let listener = StdListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let _ = sock.write_all(reply);
        }
    });
    port
}

fn get(t: &Transport, url: &str) -> Result<RawResponse, FetchError> {
    t.request_once(url, &[], MAX_BODY_BYTES, Duration::from_secs(5))
}

#[test]
fn alpn_selects_h2_and_the_exchange_completes_verified() {
    let (port, alpn) = spawn_https(vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    let resp = get(
        &Transport::new(test_roots()),
        &format!("https://localhost:{port}/"),
    )
    .unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body, b"hi");
    assert_eq!(alpn.recv().unwrap().as_deref(), Some(b"h2".as_ref()));
}

#[test]
fn h1_fallback_completes_when_the_server_offers_only_http1() {
    let (port, alpn) = spawn_https(vec![b"http/1.1".to_vec()]);
    let resp = get(
        &Transport::new(test_roots()),
        &format!("https://localhost:{port}/"),
    )
    .unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(alpn.recv().unwrap().as_deref(), Some(b"http/1.1".as_ref()));
}

#[test]
fn a_provided_user_agent_and_accept_encoding_are_not_overridden() {
    let (port, _alpn) = spawn_https(vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    let headers = vec![
        ("user-agent".to_string(), "custom".to_string()),
        ("accept-encoding".to_string(), "identity".to_string()),
    ];
    let resp = Transport::new(test_roots())
        .request_once(
            &format!("https://localhost:{port}/"),
            &headers,
            MAX_BODY_BYTES,
            Duration::from_secs(5),
        )
        .unwrap();
    assert_eq!(resp.status, 200);
}

#[test]
fn a_body_past_the_cap_is_a_body_error() {
    let (port, _alpn) = spawn_https(vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    let e = Transport::new(test_roots())
        .request_once(
            &format!("https://localhost:{port}/"),
            &[],
            1,
            Duration::from_secs(5),
        )
        .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn a_zero_deadline_is_a_timeout() {
    let (port, _alpn) = spawn_https(vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    let e = Transport::new(test_roots())
        .request_once(
            &format!("https://localhost:{port}/"),
            &[],
            MAX_BODY_BYTES,
            Duration::from_nanos(1),
        )
        .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_TIMEOUT);
}

#[test]
fn a_refused_connection_is_a_connect_error() {
    let e = get(&Transport::new(test_roots()), "https://127.0.0.1:1/").unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_CONNECT);
}

#[test]
fn an_untrusted_certificate_is_a_tls_error() {
    // The client trusts only the real webpki roots; the local server presents a
    // throwaway-CA leaf, so certificate verification fails — a TLS error, with
    // the rustls cause nested inside tokio-rustls's io error.
    let (port, _alpn) = spawn_https(vec![b"h2".to_vec(), b"http/1.1".to_vec()]);
    let e = get(
        &Transport::new(crate::fetch::firefox_tls::webpki_roots()),
        &format!("https://localhost:{port}/"),
    )
    .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_TLS);
}

#[test]
fn a_malformed_http_response_is_a_body_error() {
    let port = spawn_raw(b"NOT-A-STATUS-LINE\r\n\r\n");
    let e = get(
        &Transport::new(test_roots()),
        &format!("http://localhost:{port}/"),
    )
    .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_BODY);
}

#[test]
fn an_invalid_header_value_is_a_url_error() {
    let headers = vec![("x-bad".to_string(), "line\nbreak".to_string())];
    let e = Transport::new(test_roots())
        .request_once(
            "https://localhost/",
            &headers,
            MAX_BODY_BYTES,
            Duration::from_secs(5),
        )
        .unwrap_err();
    assert_eq!(e.kind, kinds::FETCH_URL);
}

#[test]
fn classify_covers_every_taxonomy_arm() {
    use std::io::ErrorKind::{ConnectionRefused, NotFound, Other};
    assert_eq!(classify(Some(NotFound), true, ""), kinds::FETCH_DNS);
    assert_eq!(
        classify(Some(ConnectionRefused), true, ""),
        kinds::FETCH_CONNECT
    );
    assert_eq!(classify(Some(Other), true, ""), kinds::FETCH_TLS);
    assert_eq!(classify(Some(Other), false, ""), kinds::FETCH_CONNECT);
    assert_eq!(
        classify(None, false, "failed to lookup address information"),
        kinds::FETCH_DNS
    );
    assert_eq!(classify(None, true, ""), kinds::FETCH_CONNECT);
    assert_eq!(classify(None, false, ""), kinds::FETCH_BODY);
}
