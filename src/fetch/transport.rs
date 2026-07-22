//! The async HTTP transport (Stage B): one hyper client speaking both HTTP/2 and
//! HTTP/1.1, selected by ALPN over the persona rustls config, driven by a
//! per-invocation tokio runtime with `block_on`.
//!
//! ## Lifecycle contract (Mark's async ruling, identity.md §6.1 item 2)
//!
//! The public surface here is **blocking** ([`Transport::request_once`] returns a
//! value, never a future). Internally a tokio runtime multiplexes h2 streams and
//! drives connection tasks — the concurrency Mark's ruling explicitly permits
//! ("h2 multiplexing, bl-08f6"). The runtime is owned by the [`Transport`], which
//! is owned by the per-invocation `FetchSession`, so it is **dropped when the
//! invocation ends** — no runtime outlives the call, no work escapes it.
//!
//! It is a **multi-thread** runtime, not current-thread: `run/gather.rs` fetches
//! subresources from up to six OS threads that each call the blocking API, so the
//! transport must tolerate concurrent `block_on`, which a current-thread runtime
//! cannot. The observable contract (blocking, deterministic teardown) is
//! unchanged; only the internal executor differs from the ruling's illustrative
//! "current-thread".

use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Limited};
use hyper::body::Incoming;
use hyper::Request;
use hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::{Client, Error as LegacyError};
use hyper_util::rt::TokioExecutor;
use rustls::RootCertStore;
use tokio::runtime::Runtime;

use super::firefox_tls::firefox_client_config;
use super::profile::FIREFOX_140_ESR;
use super::{FetchError, POOL_PER_HOST};
use crate::envelope::kinds;

type HttpsClient = Client<HttpsConnector<HttpConnector>, Empty<Bytes>>;

/// One raw HTTP response: status, headers (wire order preserved), and the body
/// bytes exactly as received (still content-encoded — the caller inflates).
#[derive(Debug)]
pub(crate) struct RawResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// The per-invocation transport. Holds the tokio runtime and the pooled hyper
/// client; a `FetchSession` shares one behind an `Arc`.
pub(crate) struct Transport {
    runtime: Runtime,
    client: HttpsClient,
}

impl Transport {
    /// Build the transport verifying against `roots`. The h2 SETTINGS frot can
    /// set are taken from the profile (initial window, max frame); the rest are
    /// hyper's defaults, a declared residual in the §12 oracle.
    pub(crate) fn new(roots: RootCertStore) -> Self {
        let https = HttpsConnectorBuilder(roots).build();
        let client = Client::builder(TokioExecutor::new())
            .pool_max_idle_per_host(POOL_PER_HOST)
            .http2_initial_stream_window_size(FIREFOX_140_ESR.h2.initial_window_size)
            .http2_initial_connection_window_size(FIREFOX_140_ESR.h2.connection_window_increment)
            .http2_max_frame_size(FIREFOX_140_ESR.h2.max_frame_size)
            .http1_title_case_headers(true)
            .build(https);
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio multi-thread runtime");
        Self { runtime, client }
    }

    /// GET `url` with `headers` attached, reading at most `max_body` bytes, all
    /// under one `deadline`. Blocking: the whole exchange runs to completion (or
    /// error) before returning. Follows no redirects — that is the caller's loop.
    pub(crate) fn request_once(
        &self,
        url: &str,
        headers: &[(String, String)],
        max_body: u64,
        deadline: Duration,
    ) -> Result<RawResponse, FetchError> {
        self.runtime.block_on(async {
            tokio::time::timeout(deadline, self.exchange(url, headers, max_body))
                .await
                .map_err(|_| FetchError::new(kinds::FETCH_TIMEOUT, "request timed out"))?
        })
    }

    async fn exchange(
        &self,
        url: &str,
        headers: &[(String, String)],
        max_body: u64,
    ) -> Result<RawResponse, FetchError> {
        let mut builder = Request::builder().method("GET").uri(url);
        for (n, v) in headers {
            builder = builder.header(n.as_str(), v.as_str());
        }
        // Defaults hyper does not add for us: the persona UA and the honest
        // `Accept-Encoding` (only what `decode::inflate` decodes). The request
        // derivation supplies both for a real page load; this fills them in only
        // for a bare direct call (recorder/transport tests), from the same SSOTs.
        if !has_header(headers, "user-agent") {
            builder = builder.header("user-agent", FIREFOX_140_ESR.user_agent());
        }
        if !has_header(headers, "accept-encoding") {
            builder = builder.header("accept-encoding", super::decode::ACCEPT_ENCODING);
        }
        let req = builder
            .body(Empty::<Bytes>::new())
            .map_err(|e| FetchError::new(kinds::FETCH_URL, e.to_string()))?;
        let resp = self.client.request(req).await.map_err(map_legacy_error)?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(n, v)| (n.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let body = read_body(resp.into_body(), max_body).await?;
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }
}

/// Whether `headers` already carries `name` (case-insensitive).
fn has_header(headers: &[(String, String)], name: &str) -> bool {
    headers.iter().any(|(n, _)| n.eq_ignore_ascii_case(name))
}

/// Read the response body, failing with [`kinds::FETCH_BODY`] once `max_body` is
/// exceeded (the `Limited` wrapper enforces the cap without buffering the rest).
async fn read_body(body: Incoming, max_body: u64) -> Result<Vec<u8>, FetchError> {
    let collected = Limited::new(body, max_body as usize)
        .collect()
        .await
        .map_err(|e| FetchError::new(kinds::FETCH_BODY, e.to_string()))?;
    Ok(collected.to_bytes().to_vec())
}

/// Build the hyper-rustls connector over the persona TLS config. HTTP is allowed
/// (mock/plain-HTTP tests, `http://` inputs); both TLS versions are enabled so
/// ALPN can select h2 or http/1.1.
struct HttpsConnectorBuilder(RootCertStore);

impl HttpsConnectorBuilder {
    fn build(self) -> HttpsConnector<HttpConnector> {
        hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(firefox_client_config(self.0))
            .https_or_http()
            .enable_all_versions()
            .build()
    }
}

/// Classify a hyper client error into the fetch taxonomy, message preserved.
/// hyper and tokio-rustls flatten transport failures into io errors (the rustls
/// cause is not preserved as a source), so classification reads the *deepest* io
/// error kind plus whether the failure happened at connect time.
fn map_legacy_error(e: LegacyError) -> FetchError {
    let msg = e.to_string();
    FetchError::new(classify(deepest_io_kind(&e), e.is_connect(), &msg), msg)
}

/// The kind of the deepest io error in `e`'s source chain — the most specific
/// cause once hyper has wrapped it.
fn deepest_io_kind(e: &(dyn std::error::Error + 'static)) -> Option<std::io::ErrorKind> {
    let mut kind = None;
    let mut source = Some(e);
    while let Some(s) = source {
        if let Some(io) = s.downcast_ref::<std::io::Error>() {
            kind = Some(io.kind());
        }
        source = s.source();
    }
    kind
}

/// Map (deepest io cause, connect-phase?, message) to a taxonomy kind. Pure, so
/// every arm is unit-tested without a live network: a name lookup that fails is
/// DNS; a refused/reset/timed-out peer is connect; any *other* io error during
/// the handshake is TLS (rustls flattens cert/protocol failures to an `Other` io
/// error at connect time); a connect-phase failure with no io detail is still
/// connect; anything else — a send/parse-phase error — is a malformed body.
fn classify(io: Option<std::io::ErrorKind>, is_connect: bool, msg: &str) -> &'static str {
    use std::io::ErrorKind::{
        AddrNotAvailable, ConnectionAborted, ConnectionRefused, ConnectionReset, NotFound, TimedOut,
    };
    match io {
        Some(NotFound) => kinds::FETCH_DNS,
        Some(
            ConnectionRefused | ConnectionReset | ConnectionAborted | AddrNotAvailable | TimedOut,
        ) => kinds::FETCH_CONNECT,
        Some(_) if is_connect => kinds::FETCH_TLS,
        Some(_) => kinds::FETCH_CONNECT,
        None if msg.contains("failed to lookup") => kinds::FETCH_DNS,
        None if is_connect => kinds::FETCH_CONNECT,
        None => kinds::FETCH_BODY,
    }
}

#[cfg(test)]
mod recorder;
#[cfg(test)]
mod tests;
