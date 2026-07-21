//! Drives the Firefox connector through ureq against a local `craftls` TLS
//! server (an embedded throwaway CA/leaf, so verification stays real) to cover
//! the TLS-wrap path and every `Transport` method. The `Default`/webpki and
//! skip (non-HTTPS) paths are covered by the plain-HTTP `fetch` tests.

use super::*;
use craftls::{ServerConfig, ServerConnection};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use std::io::{Read, Write};
use std::net::TcpListener;
use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::TcpConnector;
use ureq::Agent;

fn server_config() -> Arc<ServerConfig> {
    let leaf = CertificateDer::from(include_bytes!("testdata/leaf.der").to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        include_bytes!("testdata/leaf.key.der").to_vec(),
    ));
    Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![leaf], key)
            .unwrap(),
    )
}

fn test_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            include_bytes!("testdata/ca.der").to_vec(),
        ))
        .unwrap();
    roots
}

/// Wraps the real connector and formats the transport it produces, exercising
/// `FirefoxTlsTransport`'s `Debug` on a genuine handshake (no fake transport).
#[derive(Debug)]
struct DebugSpy(FirefoxTlsConnector);

impl<In: Transport> Connector<In> for DebugSpy {
    type Out = Either<In, FirefoxTlsTransport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<In>,
    ) -> Result<Option<Self::Out>, Error> {
        let out = self.0.connect(details, chained)?;
        if let Some(Either::B(t)) = &out {
            assert_eq!(format!("{t:?}"), "FirefoxTlsTransport");
        }
        Ok(out)
    }
}

#[test]
fn firefox_connector_serves_https_over_keepalive() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let sc = server_config();
    let server = std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        let conn = ServerConnection::new(sc).unwrap();
        let mut tls = StreamOwned::new(conn, &mut sock);
        let mut buf = [0u8; 2048];
        // Two requests on one pooled connection so `is_open` is exercised on reuse.
        for _ in 0..2 {
            let _ = tls.read(&mut buf).unwrap();
            tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi")
                .unwrap();
            let _ = tls.flush();
        }
    });

    let connector = ()
        .chain(TcpConnector::default())
        .chain(DebugSpy(FirefoxTlsConnector::with_roots(test_roots())));
    let agent = Agent::with_parts(
        ureq::config::Config::default(),
        connector,
        DefaultResolver::default(),
    );

    for _ in 0..2 {
        let mut res = agent
            .get(&format!("https://localhost:{port}/"))
            .call()
            .unwrap();
        assert_eq!(res.status(), 200);
        assert_eq!(res.body_mut().read_to_string().unwrap(), "hi");
    }
    server.join().unwrap();
}

fn firefox_agent() -> Agent {
    let connector =
        ().chain(TcpConnector::default())
            .chain(FirefoxTlsConnector::with_roots(test_roots()));
    Agent::with_parts(
        ureq::config::Config::default(),
        connector,
        DefaultResolver::default(),
    )
}

/// A peer that accepts the connection then resets it mid-handshake surfaces as
/// a transport error, not a hang or a panic: the TLS stream's read/write are
/// the frot-owned seam that error travels through.
#[test]
fn a_peer_that_resets_mid_handshake_is_a_transport_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for sock in listener.incoming().flatten() {
            // Let the ClientHello land, then close without ever reading it.
            // Closing a socket with unread data queued forces an RST, so the
            // client sees a reset rather than a clean EOF.
            std::thread::sleep(std::time::Duration::from_millis(50));
            drop(sock);
        }
    });
    let err = firefox_agent()
        .get(&format!("https://localhost:{port}/"))
        .call()
        .unwrap_err();
    assert!(
        matches!(err, Error::Io(_) | Error::Tls(_) | Error::ConnectionFailed),
        "unexpected error: {err:?}"
    );
}

/// An IPv6-literal authority reaches the connector as the bracketed form
/// `[::1]`, which is not a valid DNS name. That must be a clean `Tls` error
/// rather than a panic inside the connector.
#[test]
fn an_ipv6_literal_authority_is_rejected_as_a_dns_name() {
    let listener = TcpListener::bind("[::1]:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || for _s in listener.incoming().flatten() {});
    let err = firefox_agent()
        .get(&format!("https://[::1]:{port}/"))
        .call()
        .unwrap_err();
    assert!(
        matches!(err, Error::Tls(m) if m == "invalid dns name"),
        "unexpected"
    );
}

/// `ClientConnection::new` can refuse the config itself, before a single byte
/// reaches the wire. `connect` must turn that into the same clean `Tls` error
/// as any other handshake refusal rather than unwrapping. Driven with the real
/// shipping config and one field rustls rejects — `max_fragment_size` outside
/// the accepted 32..=16389 — so what is under test is the production
/// construction path, not a lookalike config assembled for the test.
#[test]
fn a_config_the_connection_rejects_is_a_tls_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || for _s in listener.incoming().flatten() {});

    let mut config = firefox_config(test_roots());
    config.max_fragment_size = Some(16);
    let connector =
        ().chain(TcpConnector::default())
            .chain(FirefoxTlsConnector::with_config(config));
    let agent = Agent::with_parts(
        ureq::config::Config::default(),
        connector,
        DefaultResolver::default(),
    );

    let err = agent
        .get(&format!("https://localhost:{port}/"))
        .call()
        .unwrap_err();
    assert!(
        matches!(err, Error::Tls(m) if m == "tls client connection"),
        "unexpected error: {err:?}"
    );
}

/// A peer that completes the handshake and then emits bytes that are not valid
/// TLS records fails the *read* side of the stream. That is a distinct seam
/// from a reset during connect: here the transport is live and the corruption
/// surfaces out of `await_input`.
#[test]
fn garbage_after_a_good_handshake_fails_the_read_side() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let sc = server_config();
    std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        {
            let conn = ServerConnection::new(sc).unwrap();
            let mut tls = StreamOwned::new(conn, &mut sock);
            let _ = tls.read(&mut [0u8; 2048]);
        }
        // Raw, un-encrypted noise where a TLS record should be.
        let _ = sock.write_all(&[0xff; 256]);
    });
    let err = firefox_agent()
        .get(&format!("https://localhost:{port}/"))
        .call()
        .unwrap_err();
    assert!(
        matches!(err, Error::Io(_) | Error::Tls(_)),
        "unexpected error: {err:?}"
    );
}

/// Persona contract, the HTTPS half (identity.md §12 — deterministic oracle for
/// `bl-46f5`). Two recorded facts about frot's wire identity, captured through
/// the real connector with no network:
/// - **§3.4 casing:** over `https://` the request head is Title-Cased (`Host:`,
///   `User-Agent:`), never the lowercase `http://` tell. `bl-20ec` unifies casing
///   across schemes and updates this assertion.
/// - **§3.1 ALPN:** frot advertises `http/1.1` only, so against a server offering
///   both `h2` and `http/1.1` the negotiated protocol is `http/1.1`. `bl-abca`
///   moves this to `h2` and updates the expected value here.
#[test]
fn https_persona_head_is_title_cased_and_alpn_is_http1() {
    let leaf = CertificateDer::from(include_bytes!("testdata/leaf.der").to_vec());
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        include_bytes!("testdata/leaf.key.der").to_vec(),
    ));
    let mut config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![leaf], key)
        .unwrap();
    // Offer both protocols; frot's ClientHello advertises only http/1.1, so h1
    // must win — that is the assertion.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    let sc = Arc::new(config);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let (mut sock, _) = listener.accept().unwrap();
        let conn = ServerConnection::new(sc).unwrap();
        let mut tls = StreamOwned::new(conn, &mut sock);
        let mut buf = [0u8; 4096];
        let n = tls.read(&mut buf).unwrap();
        let text = String::from_utf8_lossy(&buf[..n]).to_string();
        let head = text.split("\r\n\r\n").next().unwrap().to_string();
        let alpn = tls.conn.alpn_protocol().map(<[u8]>::to_vec);
        tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nhi")
            .unwrap();
        let _ = tls.flush();
        tx.send((head, alpn)).unwrap();
    });

    let connector =
        ().chain(TcpConnector::default())
            .chain(FirefoxTlsConnector::with_roots(test_roots()));
    let config = Agent::config_builder()
        .user_agent(crate::fetch::user_agent(&[]))
        .build();
    let agent = Agent::with_parts(config, connector, DefaultResolver::default());
    let mut res = agent
        .get(&format!("https://localhost:{port}/"))
        .call()
        .unwrap();
    assert_eq!(res.status(), 200);
    let _ = res.body_mut().read_to_string();

    let (head, alpn) = rx.recv().unwrap();
    assert!(
        head.contains("\r\nHost:"),
        "Host must be Title-Cased: {head}"
    );
    assert!(
        head.contains("User-Agent: Mozilla/5.0 (X11; Linux x86_64;"),
        "User-Agent Title-Cased persona UA: {head}"
    );
    assert!(
        !head.contains("\r\nhost:") && !head.contains("\r\nuser-agent:"),
        "no lowercase header tell over https: {head}"
    );
    assert_eq!(alpn.as_deref(), Some(b"http/1.1".as_ref()));
}
