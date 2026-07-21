//! Document fetcher: blocking `ureq`-based GET with sane browser-ish
//! defaults, plus `file://` reads for documents the caller already has.
//!
//! Returns the final URL (post-redirect), status (`None` for `file://` — no
//! HTTP response happened), headers, decoded body, and the charset that was
//! used to decode it. Errors map to the canonical [`crate::envelope::kinds`]
//! taxonomy that the envelope serializer expects.
//!
//! Every request rides one per-invocation [`FetchSession`] (`session.rs`): the
//! session owns the connection pool, the caller header overrides, and the
//! invocation-local resource cache. The transport (`ureq::Agent`) is built
//! *once*, in [`build_agent`], and reached only through the session — no call
//! site constructs its own transport, so a page load reuses connections and
//! presents its TLS identity once, not once per request.
//!
//! `file://` support means frot can reach local disk: callers passing
//! untrusted URLs should validate the scheme themselves, same as with curl.

use std::time::Duration;

use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::{Connector, TcpConnector};
use ureq::{Agent, ResponseExt};

use crate::envelope::kinds;

mod decode;
mod firefox_tls;
mod session;

pub(crate) use decode::decode_body;
pub use session::{FetchSession, Intent};

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";

pub(crate) const TIMEOUT_SECS: u64 = 15;
pub(crate) const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

/// Idle HTTP/1.1 connections kept warm per host on the shared pool. Six is
/// Firefox's `network.http.max-persistent-connections-per-server`, the same
/// limit the CSS gather's `MAX_IN_FLIGHT` cites — a full wave of subresources to
/// one origin stays warm for reuse, and frot never keeps more open than the
/// browser it presents as.
pub(crate) const POOL_PER_HOST: usize = 6;

/// The User-Agent frot sends: a `-H "User-Agent: …"` override when the caller
/// supplied one, else the built-in default. `navigator.userAgent` (js.md §7)
/// reports the same string, so the shim never lies about who fetched.
pub fn user_agent(headers: &[(String, String)]) -> &str {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case("user-agent"))
        .map_or(USER_AGENT, |(_, v)| v.as_str())
}

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub final_url: String,
    /// HTTP status; `None` when no response happened (`file://`).
    pub status: Option<u16>,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub charset: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchError {
    pub kind: String,
    pub message: String,
}

impl FetchError {
    pub fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_string(),
            message: message.into(),
        }
    }
}

/// Build the one per-invocation transport: the browser-identity connector chain
/// (TCP, then a Firefox-shaped TLS handshake for HTTPS — `firefox_tls`) over a
/// shared connection pool. `http_status_as_error(false)` keeps a 4xx/5xx body
/// readable (the envelope surfaces the code, `run.rs`); the per-request timeout
/// is applied per call in [`dispatch`], so one agent serves every request under
/// its own deadline. Called exactly once, by [`FetchSession::new`].
pub(crate) fn build_agent() -> Agent {
    let config = Agent::config_builder()
        .user_agent(USER_AGENT)
        .http_status_as_error(false)
        .max_idle_connections_per_host(POOL_PER_HOST)
        .build();
    let connector = ()
        .chain(TcpConnector::default())
        .chain(firefox_tls::FirefoxTlsConnector::default());
    Agent::with_parts(config, connector, DefaultResolver::default())
}

/// GET `url` over the session's shared `agent` with `headers` attached, under
/// `timeout`. `headers` is the fully-resolved request set the session prepared
/// (nav headers layered, or same-origin `-H` scoped) — a `User-Agent` among
/// them overrides the agent's default (ureq applies its config UA only when the
/// request carries none), so the caller's `-H "User-Agent"` still wins.
/// `Authorization` is never forwarded across redirects (ureq's default).
/// `file://` reads take the read path and ignore `headers` / the pool.
pub(crate) fn dispatch(
    agent: &Agent,
    url: &str,
    headers: &[(String, String)],
    timeout: Duration,
) -> Result<FetchResult, FetchError> {
    let parsed = validate_url(url)?;
    if parsed.scheme() == "file" {
        return fetch_file(&parsed);
    }
    let mut request = agent.get(url).config().timeout_global(Some(timeout)).build();
    for (n, v) in headers {
        request = request.header(n.as_str(), v.as_str());
    }
    let mut response = request.call().map_err(map_ureq_error)?;
    let final_url = response.get_uri().to_string();
    let status = response.status().as_u16();

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(n, v)| (n.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    let content_type = header_value(&headers, "content-type");
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_vec()
        .map_err(map_ureq_error)?;
    let (body, charset) = decode_body(&bytes, content_type.as_deref());
    Ok(FetchResult {
        final_url,
        status: Some(status),
        headers,
        body,
        charset,
    })
}

/// Read a `file://` document. No response, so no status and no headers;
/// charset comes from the `<meta>` sniff (or UTF-8), same as a header-less
/// HTTP response.
pub(crate) fn fetch_file(url: &url::Url) -> Result<FetchResult, FetchError> {
    let path = url.to_file_path().map_err(|()| {
        FetchError::new(kinds::FETCH_URL, format!("not a local file path: {}", url))
    })?;
    let len = std::fs::metadata(&path)
        .map_err(|e| FetchError::new(kinds::FETCH_FILE, format!("{}: {}", path.display(), e)))?
        .len();
    check_body_len(len)?;
    let bytes = std::fs::read(&path)
        .map_err(|e| FetchError::new(kinds::FETCH_FILE, format!("{}: {}", path.display(), e)))?;
    let (body, charset) = decode_body(&bytes, None);
    Ok(FetchResult {
        final_url: url.to_string(),
        status: None,
        headers: Vec::new(),
        body,
        charset,
    })
}

/// The one body-size rule, shared by both transports.
pub(crate) fn check_body_len(len: u64) -> Result<(), FetchError> {
    if len > MAX_BODY_BYTES {
        return Err(FetchError::new(
            kinds::FETCH_BODY,
            format!("body is {} bytes; limit is {}", len, MAX_BODY_BYTES),
        ));
    }
    Ok(())
}

/// Whether two URLs share an origin (scheme + host + port, with default
/// ports normalized). Unparseable input is never same-origin.
pub fn same_origin(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (url::Url::parse(a), url::Url::parse(b)) else {
        return false;
    };
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

pub(crate) fn validate_url(url: &str) -> Result<url::Url, FetchError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| FetchError::new(kinds::FETCH_URL, format!("invalid URL: {}", e)))?;
    if !matches!(parsed.scheme(), "http" | "https" | "file") {
        return Err(FetchError::new(
            kinds::FETCH_URL,
            format!("unsupported scheme: {}", parsed.scheme()),
        ));
    }
    Ok(parsed)
}

/// Case-insensitive response-header lookup, shared with `needs::challenge`.
pub(crate) fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

pub(crate) fn map_ureq_error(e: ureq::Error) -> FetchError {
    use ureq::Error;
    let msg = e.to_string();
    let kind = match &e {
        Error::HostNotFound => kinds::FETCH_DNS,
        Error::ConnectionFailed => kinds::FETCH_CONNECT,
        Error::Timeout(_) => kinds::FETCH_TIMEOUT,
        Error::TooManyRedirects | Error::RedirectFailed => kinds::FETCH_REDIRECT,
        Error::BadUri(_) | Error::RequireHttpsOnly(_) => kinds::FETCH_URL,
        Error::Tls(_) => kinds::FETCH_TLS,
        Error::Io(_)
        | Error::BodyExceedsLimit(_)
        | Error::BodyStalled
        | Error::Decompress(_, _) => kinds::FETCH_BODY,
        _ => kinds::INTERNAL,
    };
    FetchError::new(kind, msg)
}

#[cfg(test)]
mod tests;
