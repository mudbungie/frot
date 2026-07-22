//! Wire recorders (§12 oracle): drive the production [`Transport`] with the
//! derived header set and assert the exact ordered header set each serializer
//! puts on the wire, for all five intents and same-/cross-origin variants.
//!
//! Division of labour: `tests.rs` proves the *derivation* is correct (values
//! against hardcoded expectations); these recorders prove the *serializers*
//! transmit that derivation faithfully — h1 in Firefox order with `Host`
//! appended last and `Te` present, h2 lowercased with `:authority` synthesised
//! from the URI and `te` kept last.

use std::io::{Read, Write};
use std::net::TcpListener;
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
use rustls::ServerConfig;
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio::net::TcpListener as TokioListener;
use tokio_rustls::TlsAcceptor;

use super::derive_headers;
use crate::fetch::firefox_tls::webpki_roots;
use crate::fetch::{Intent, Transport, MAX_BODY_BYTES};

const GENEROUS: Duration = Duration::from_secs(5);

/// One h2 capture: the request `:authority` and its ordered header pairs.
type H2Capture = (String, Vec<(String, String)>);

/// How a case's referrer source relates to the target: a user navigation (none),
/// a same-origin subresource, or a cross-site one.
#[derive(Clone, Copy)]
enum Rel {
    Nav,
    Same,
    Cross,
}

/// The five intents across their origin variants — the acceptance matrix.
const CASES: &[(Intent, Rel)] = &[
    (Intent::Navigation, Rel::Nav),
    (Intent::Style, Rel::Same),
    (Intent::Style, Rel::Cross),
    (Intent::ClassicScript, Rel::Same),
    (Intent::ClassicScript, Rel::Cross),
    (Intent::Module, Rel::Same),
    (Intent::Module, Rel::Cross),
    (Intent::FetchXhr, Rel::Same),
    (Intent::FetchXhr, Rel::Cross),
];

/// The page (caller-`-H` anchor and, for a subresource, referrer source) for a
/// case, given the target's own `origin` (`scheme://authority`). Cross-site uses
/// a stable foreign host so its origin-only referer needs no normalization.
fn page(rel: Rel, origin: &str) -> Option<String> {
    match rel {
        Rel::Nav => None,
        Rel::Same => Some(format!("{origin}/pg")),
        Rel::Cross => Some(format!("{}//cross.test/pg", scheme_of(origin))),
    }
}

fn scheme_of(origin: &str) -> &str {
    &origin[..origin.find(':').unwrap() + 1]
}

/// hyper's h1 Title-Case of a header name (`te` → `Te`, `user-agent` →
/// `User-Agent`): capitalize each `-`-delimited token.
fn title_case(name: &str) -> String {
    name.split('-')
        .map(|t| {
            let mut c = t.chars();
            c.next()
                .map(|f| f.to_ascii_uppercase().to_string() + &c.as_str().to_ascii_lowercase())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("-")
}

/// Replace the loopback authority with `HOST`, so a captured head is byte-stable.
fn norm(s: &str, authority: &str) -> String {
    s.replace(authority, "HOST")
}

/// The h1 wire expectation: the derived set Title-Cased in order, then the
/// hyper-synthesised `Host` last.
fn expect_h1(
    intent: Intent,
    target: &str,
    page: Option<&str>,
    authority: &str,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = derive_headers(intent, target, target, page, &[], None)
        .iter()
        .map(|(n, v)| (title_case(n), norm(v, authority)))
        .collect();
    out.push(("Host".to_string(), "HOST".to_string()));
    out
}

/// The h2 wire expectation: the derived set lowercased in order (`:authority` is
/// synthesised from the URI and asserted separately; the derivation sends no
/// `Host`/`Connection`, so nothing is stripped and `te` stays last).
fn expect_h2(
    intent: Intent,
    target: &str,
    page: Option<&str>,
    authority: &str,
) -> Vec<(String, String)> {
    derive_headers(intent, target, target, page, &[], None)
        .iter()
        .map(|(n, v)| (n.to_ascii_lowercase(), norm(v, authority)))
        .collect()
}

/// A one-shot plain-HTTP/1.1 origin: captures the request head and replies 200.
fn h1_origin() -> (u16, Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        let mut buf = [0u8; 4096];
        let n = sock.read(&mut buf).unwrap();
        let text = String::from_utf8_lossy(&buf[..n]).to_string();
        sock.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi")
            .unwrap();
        let _ = sock.flush();
        tx.send(text.split("\r\n\r\n").next().unwrap().to_string())
            .unwrap();
    });
    (port, rx)
}

/// Parse a captured head into ordered `(name, value)` pairs, dropping the
/// request line.
fn parse_head(head: &str) -> Vec<(String, String)> {
    head.lines()
        .skip(1)
        .filter_map(|l| l.split_once(": "))
        .map(|(n, v)| (n.to_string(), v.to_string()))
        .collect()
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

/// A one-shot h2-over-TLS origin: captures the first request's `:authority` and
/// ordered headers, replies 200.
fn h2_origin() -> (u16, Receiver<H2Capture>) {
    let (port_tx, port_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TokioListener::bind("127.0.0.1:0").await.unwrap();
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();
            let acceptor = TlsAcceptor::from(server_config());
            let (tcp, _) = listener.accept().await.unwrap();
            let tls = acceptor.accept(tcp).await.unwrap();
            let svc = service_fn(move |req: Request<Incoming>| {
                let tx = tx.clone();
                async move {
                    let authority = req
                        .uri()
                        .authority()
                        .map(|a| a.to_string())
                        .unwrap_or_default();
                    let hs = req
                        .headers()
                        .iter()
                        .map(|(n, v)| {
                            (n.as_str().to_string(), v.to_str().unwrap_or("").to_string())
                        })
                        .collect();
                    tx.send((authority, hs)).unwrap();
                    Ok::<_, std::convert::Infallible>(Response::new(Full::new(Bytes::from_static(
                        b"hi",
                    ))))
                }
            });
            let _ = auto::Builder::new(TokioExecutor::new())
                .serve_connection(TokioIo::new(tls), svc)
                .await;
        });
    });
    (port_rx.recv().unwrap(), rx)
}

#[test]
fn h1_recorder_matches_the_derivation_for_every_intent() {
    for &(intent, rel) in CASES {
        let (port, rx) = h1_origin();
        let authority = format!("127.0.0.1:{port}");
        let target = format!("http://{authority}/t");
        let origin = format!("http://{authority}");
        let page = page(rel, &origin);
        let headers = derive_headers(intent, &target, &target, page.as_deref(), &[], None);
        let _ = Transport::new(webpki_roots()).request_once(
            &target,
            &headers,
            MAX_BODY_BYTES,
            GENEROUS,
        );
        let head = rx.recv().unwrap();
        let got: Vec<(String, String)> = parse_head(&head)
            .into_iter()
            .map(|(n, v)| (n, norm(&v, &authority)))
            .collect();
        assert_eq!(got, expect_h1(intent, &target, page.as_deref(), &authority));
    }
}

#[test]
fn h2_recorder_matches_the_derivation_for_every_intent() {
    for &(intent, rel) in CASES {
        let (port, rx) = h2_origin();
        let authority = format!("localhost:{port}");
        let target = format!("https://{authority}/t");
        let origin = format!("https://{authority}");
        let page = page(rel, &origin);
        let headers = derive_headers(intent, &target, &target, page.as_deref(), &[], None);
        let _ =
            Transport::new(test_roots()).request_once(&target, &headers, MAX_BODY_BYTES, GENEROUS);
        let (got_authority, hs) = rx.recv().unwrap();
        let got: Vec<(String, String)> = hs
            .into_iter()
            .map(|(n, v)| (n, norm(&v, &authority)))
            .collect();
        assert_eq!(norm(&got_authority, &authority), "HOST");
        assert_eq!(got, expect_h2(intent, &target, page.as_deref(), &authority));
    }
}

/// The throwaway CA the h2 origin's leaf chains to, so the client verifies it.
fn test_roots() -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            include_bytes!("../firefox_tls/testdata/ca.der").to_vec(),
        ))
        .unwrap();
    roots
}
