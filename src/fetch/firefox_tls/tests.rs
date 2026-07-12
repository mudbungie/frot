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
        .add(CertificateDer::from(include_bytes!("testdata/ca.der").to_vec()))
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
