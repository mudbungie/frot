//! Document fetcher: blocking GET over the hyper h1+h2 transport with sane
//! browser-ish defaults, plus `file://` reads for documents the caller has.
//!
//! Returns the final URL (post-redirect), status (`None` for `file://` — no
//! HTTP response happened), headers, decoded body, and the charset that was
//! used to decode it. Errors map to the canonical [`crate::envelope::kinds`]
//! taxonomy that the envelope serializer expects.
//!
//! Every request rides one per-invocation [`FetchSession`] (`session.rs`): the
//! session owns the connection pool (one hyper client + tokio runtime), the
//! caller header overrides, and the invocation-local resource cache. The
//! transport is built *once*, in [`build_transport`], and reached only through
//! the session — no call site constructs its own transport, so a page load
//! reuses connections (multiplexed over h2) and presents its TLS identity once.
//!
//! Redirects, content decoding and the error taxonomy are frot's here (the
//! transport does one exchange); `file://` support means frot can reach local
//! disk: callers passing untrusted URLs should validate the scheme themselves.

use std::time::Duration;

use crate::envelope::kinds;

mod decode;
mod firefox_tls;
mod profile;
mod session;
mod transport;

pub(crate) use decode::decode_body;
pub use profile::{BrowserProfile, H2Profile, TlsProfile, FIREFOX_140_ESR};
pub use session::{FetchSession, Intent};
pub(crate) use transport::Transport;

/// Redirects followed before giving up with [`kinds::FETCH_REDIRECT`] — ureq's
/// old default, so behaviour is unchanged.
const MAX_REDIRECTS: usize = 10;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";

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

/// Build the one per-invocation transport: the hyper h1+h2 client over the
/// persona TLS config, with the shared connection pool and its tokio runtime.
/// Called exactly once, by [`FetchSession::new`].
pub(crate) fn build_transport() -> Transport {
    Transport::new(firefox_tls::webpki_roots())
}

/// GET `url` over the session's shared `transport` with `headers` attached, each
/// exchange under `timeout`. `headers` is the fully-resolved request set the
/// session prepared (nav headers layered, or same-origin `-H` scoped). Follows
/// up to [`MAX_REDIRECTS`] redirects, stripping `Authorization` when a redirect
/// crosses origin. `file://` reads take the read path and ignore the pool.
pub(crate) fn dispatch(
    transport: &Transport,
    url: &str,
    headers: &[(String, String)],
    timeout: Duration,
) -> Result<FetchResult, FetchError> {
    let parsed = validate_url(url)?;
    if parsed.scheme() == "file" {
        return fetch_file(&parsed);
    }
    let mut current = url.to_string();
    let mut headers = headers.to_vec();
    for _ in 0..MAX_REDIRECTS {
        let resp = transport.request_once(&current, &headers, MAX_BODY_BYTES, timeout)?;
        match redirect_target(resp.status, &resp.headers, &current)? {
            Some(next) => {
                if !same_origin(&current, &next) {
                    headers.retain(|(n, _)| !n.eq_ignore_ascii_case("authorization"));
                }
                current = next;
            }
            None => {
                let content_type = header_value(&resp.headers, "content-type");
                let bytes = decode::inflate(&resp.headers, resp.body)?;
                let (body, charset) = decode_body(&bytes, content_type.as_deref());
                return Ok(FetchResult {
                    final_url: current,
                    status: Some(resp.status),
                    headers: resp.headers,
                    body,
                    charset,
                });
            }
        }
    }
    Err(FetchError::new(kinds::FETCH_REDIRECT, "too many redirects"))
}

/// The absolute redirect target for a response, or `None` when it is terminal.
/// A 3xx with a resolvable `Location` redirects; a 3xx without one, or an
/// unresolvable `Location`, is an error rather than a silent stop.
fn redirect_target(
    status: u16,
    headers: &[(String, String)],
    base: &str,
) -> Result<Option<String>, FetchError> {
    if !matches!(status, 301 | 302 | 303 | 307 | 308) {
        return Ok(None);
    }
    let location = header_value(headers, "location")
        .ok_or_else(|| FetchError::new(kinds::FETCH_REDIRECT, "redirect without Location"))?;
    let next = url::Url::parse(base)
        .and_then(|b| b.join(&location))
        .map_err(|e| FetchError::new(kinds::FETCH_REDIRECT, format!("bad Location: {e}")))?;
    Ok(Some(next.to_string()))
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

#[cfg(test)]
mod tests;
