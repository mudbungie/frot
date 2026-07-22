//! The h2 multiplexing oracle (`bl-08f6`): a wave of concurrent requests rides
//! **one** h2 connection as concurrent streams — the browser-like behaviour that
//! closes identity.md §3.5's "one TCP connection per request" defect. Proven by
//! a barrier rendezvous plus a connection count, no timing threshold. The TLS
//! server is the throwaway-CA leaf `tests.rs` also uses; helpers are duplicated
//! here so the module stands alone.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
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

use super::{RawResponse, Transport};
use crate::fetch::{FetchError, MAX_BODY_BYTES};

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

fn get(t: &Transport, url: &str) -> Result<RawResponse, FetchError> {
    t.request_once(url, &[], MAX_BODY_BYTES, Duration::from_secs(5))
}

/// An h2-only TLS origin counting accepted TCP connections; every request except
/// `/warm` blocks on a shared async [`tokio::sync::Barrier`], so it can answer
/// only when `parties` requests are in flight at once.
fn spawn_h2_barrier(parties: usize) -> (u16, Arc<AtomicUsize>) {
    let (port_tx, port_rx) = mpsc::channel();
    let conns = Arc::new(AtomicUsize::new(0));
    let counter = conns.clone();
    thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();
            let acceptor = TlsAcceptor::from(server_config(vec![b"h2".to_vec()]));
            let barrier = Arc::new(tokio::sync::Barrier::new(parties));
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                counter.fetch_add(1, Ordering::SeqCst);
                let (acceptor, barrier) = (acceptor.clone(), barrier.clone());
                tokio::spawn(async move {
                    let Ok(tls) = acceptor.accept(tcp).await else {
                        return;
                    };
                    let svc = service_fn(move |req: Request<Incoming>| {
                        let barrier = barrier.clone();
                        async move {
                            if req.uri().path() != "/warm" {
                                barrier.wait().await;
                            }
                            let body = Full::new(Bytes::from_static(b"hi"));
                            Ok::<_, std::convert::Infallible>(Response::new(body))
                        }
                    });
                    let _ = auto::Builder::new(TokioExecutor::new())
                        .serve_connection(TokioIo::new(tls), svc)
                        .await;
                });
            }
        });
    });
    (port_rx.recv().unwrap(), conns)
}

#[test]
fn h2_multiplexes_concurrent_streams_on_one_connection() {
    // Four requests each wedge on a barrier needing all four present, so they
    // return only if they ran as four concurrent h2 streams — and the single
    // accepted connection proves they multiplexed onto one socket, not four. No
    // timing threshold: a rendezvous and a count.
    let parties = 4;
    let (port, conns) = spawn_h2_barrier(parties);
    let transport = Arc::new(Transport::new(test_roots()));
    // Establish the pooled h2 connection first, so the wave multiplexes over it
    // rather than racing cold-start connections.
    get(&transport, &format!("https://localhost:{port}/warm")).unwrap();
    let url = format!("https://localhost:{port}/");
    let handles: Vec<_> = (0..parties)
        .map(|_| {
            let (t, u) = (transport.clone(), url.clone());
            thread::spawn(move || get(&t, &u).unwrap().body)
        })
        .collect();
    for h in handles {
        assert_eq!(h.join().unwrap(), b"hi");
    }
    assert_eq!(conns.load(Ordering::SeqCst), 1);
}
