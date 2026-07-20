//! Firefox TLS ClientHello for the fetch transport.
//!
//! WAFs (e.g. StackOverflow's) fingerprint the TLS ClientHello (JA3/JA4) and
//! 403 clients whose handshake is not a real browser's — stock rustls emits a
//! fixed, GREASE-less, non-Firefox ClientHello and gets refused even with
//! perfect Firefox request headers. This module makes frot's HTTPS handshake
//! *look like Firefox* (cipher/extension order, GREASE, key-share, padding),
//! the transport half of the browser masquerade `navigator`/UA already do.
//!
//! It is a narrow seam: a [`ureq`] `Connector` that wraps the chained TCP
//! transport in a `craftls` (a rustls fork with a craftable ClientHello) stream
//! carrying the `FIREFOX_105` fingerprint. Everything else — HTTP/1.1,
//! redirects, gzip/brotli, timeouts, the error taxonomy — stays ureq's. Only
//! the handshake bytes change, so `craftls` never leaks past this file (the
//! same discipline as the JS engine behind `src/js/engine.rs`).
//!
//! Matching a browser fingerprint is in scope (VISION: "repping a browser");
//! CAPTCHA / anti-bot JS-challenge solving is the refused boundary and is not
//! attempted here.

use std::sync::Arc;

use craftls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use rustls_pki_types::ServerName;
use ureq::unversioned::transport::{
    Buffers, ConnectionDetails, Connector, Either, LazyBuffers, NextTimeout, Transport,
    TransportAdapter,
};
use ureq::Error;

/// A `ureq` connector that TLS-wraps the chained transport with a Firefox
/// ClientHello. Holds the shared client config (built once).
#[derive(Debug)]
pub struct FirefoxTlsConnector {
    config: Arc<ClientConfig>,
}

impl Default for FirefoxTlsConnector {
    /// The shipping connector: trust the Mozilla webpki root set (the same
    /// roots ureq's own rustls connector defaults to).
    fn default() -> Self {
        Self::with_roots(webpki_roots())
    }
}

fn webpki_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots
}

impl FirefoxTlsConnector {
    /// Build a connector verifying against `roots`. `Default` uses the webpki
    /// set; the tests inject a throwaway CA so a local server can be reached
    /// without disabling verification.
    pub fn with_roots(roots: RootCertStore) -> Self {
        Self::with_config(firefox_config(roots))
    }

    /// Build a connector over an already-assembled `config`. Whether a
    /// per-connection [`ClientConnection`] can be constructed at all is decided
    /// entirely by the config, so it arrives through the signature rather than
    /// being welded into the constructor — that is what lets a test drive
    /// `connect`'s construction-refusal arm with the shipping config plus one
    /// rejected field, instead of leaving the arm unreachable by design.
    pub fn with_config(config: ClientConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

/// The shipping client config: Firefox's ClientHello over `roots`.
fn firefox_config(roots: RootCertStore) -> ClientConfig {
    ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
        // `test_alpn_http1`: the Firefox extension set, but ALPN offers only
        // http/1.1 — ureq speaks HTTP/1.1, so we must not let the server
        // select h2. (SO clears on this variant; h2 is future work.)
        .with_fingerprint(craftls::craft::FIREFOX_105.test_alpn_http1.builder())
}

impl<In: Transport> Connector<In> for FirefoxTlsConnector {
    type Out = Either<In, FirefoxTlsTransport>;

    fn connect(
        &self,
        details: &ConnectionDetails,
        chained: Option<In>,
    ) -> Result<Option<Self::Out>, Error> {
        let transport = chained.expect("firefox tls connector requires a chained transport");
        // Non-HTTPS (or already-TLS) transports pass through untouched.
        if !details.needs_tls() || transport.is_tls() {
            return Ok(Some(Either::A(transport)));
        }
        let name: ServerName<'static> = details
            .uri
            .authority()
            .expect("uri authority for tls")
            .host()
            .to_string()
            .try_into()
            .map_err(|_| Error::Tls("invalid dns name"))?;
        let conn = ClientConnection::new(self.config.clone(), name)
            .map_err(|_| Error::Tls("tls client connection"))?;
        let stream = StreamOwned {
            conn,
            sock: TransportAdapter::new(transport.boxed()),
        };
        let buffers = LazyBuffers::new(
            details.config.input_buffer_size(),
            details.config.output_buffer_size(),
        );
        Ok(Some(Either::B(FirefoxTlsTransport { buffers, stream })))
    }
}

/// Title-Case the header names of an HTTP/1.1 request head (`user-agent` ->
/// `User-Agent`) so the on-wire request reads like a browser's. The request
/// line (line 0) and the blank terminator pass through untouched; header values
/// (everything after the first `:`) are left exactly as ureq wrote them.
fn title_case_headers(buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(buf.len());
    for (idx, line) in buf.split_inclusive(|&b| b == b'\n').enumerate() {
        let colon = line.iter().position(|&b| b == b':');
        if let (true, Some(i)) = (idx > 0, colon) {
            let mut upper = true;
            for &b in &line[..i] {
                out.push(if upper { b.to_ascii_uppercase() } else { b });
                upper = b == b'-';
            }
            out.extend_from_slice(&line[i..]);
        } else {
            out.extend_from_slice(line);
        }
    }
    out
}

/// The TLS transport ureq drives once the handshake is wrapped. Mirrors ureq's
/// own `RustlsTransport`: buffer plumbing over the `craftls` stream.
pub struct FirefoxTlsTransport {
    buffers: LazyBuffers,
    stream: StreamOwned<ClientConnection, TransportAdapter>,
}

// `craftls` types are not `Debug`, so the `Transport: Debug` bound is satisfied
// by hand (ureq's `RustlsTransport` does the same).
impl std::fmt::Debug for FirefoxTlsTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FirefoxTlsTransport")
    }
}

impl Transport for FirefoxTlsTransport {
    fn buffers(&mut self) -> &mut dyn Buffers {
        &mut self.buffers
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), Error> {
        use std::io::Write;
        self.stream.sock.set_timeout(timeout);
        // ureq (via `http`) serializes header names lowercase; browsers send
        // Title-Case, and WAFs treat all-lowercase names as a bot tell (SO 403s
        // on it even behind a Firefox ClientHello). frot only ever GETs, so the
        // output is the request head — Title-Case the header names on the wire.
        let bytes = title_case_headers(&self.buffers.output()[..amount]);
        self.stream.write_all(&bytes)?;
        Ok(())
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, Error> {
        use std::io::Read;
        self.stream.sock.set_timeout(timeout);
        let input = self.buffers.input_append_buf();
        let amount = self.stream.read(input)?;
        self.buffers.input_appended(amount);
        Ok(amount > 0)
    }

    fn is_open(&mut self) -> bool {
        self.stream.sock.get_mut().is_open()
    }

    fn is_tls(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests;
