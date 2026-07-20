//! Document fetcher: blocking `ureq`-based GET with sane browser-ish
//! defaults, plus `file://` reads for documents the caller already has.
//!
//! Returns the final URL (post-redirect), status (`None` for `file://` — no
//! HTTP response happened), headers, decoded body, and the charset that was
//! used to decode it. Errors map to the canonical [`crate::envelope::kinds`]
//! taxonomy that the envelope serializer expects.
//!
//! `file://` support means frot can reach local disk: callers passing
//! untrusted URLs should validate the scheme themselves, same as with curl.

use std::time::Duration;

use ureq::unversioned::resolver::DefaultResolver;
use ureq::unversioned::transport::{Connector, TcpConnector};
use ureq::{Agent, ResponseExt};

use crate::envelope::kinds;

mod firefox_tls;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";

/// Firefox-121 document-navigation default headers, layered under the caller's
/// `-H` on the top-level page GET so the request shape matches the
/// [`USER_AGENT`] (and `navigator`) it claims. A bare Firefox UA with
/// `accept: */*` and no `Accept-Language`/`Sec-Fetch-*` is an obvious bot tell;
/// this is consistency of the existing masquerade, not anti-bot evasion. Pinned
/// to the same Firefox version as `USER_AGENT` — bump them together.
///
/// `Accept-Encoding` is deliberately absent: ureq sets it from its enabled
/// decoders (`gzip, br`). Real Firefox also advertises `deflate`, but we only
/// claim what we can actually inflate — a false `deflate` would break decoding
/// of a deflate-encoded body. Subresource subfetches (`docs/design/js.md` §6)
/// do NOT carry this set: navigate-mode `Sec-Fetch-*` on a subresource is
/// itself inconsistent, so they stay honest-minimal.
const DOCUMENT_HEADERS: &[(&str, &str)] = &[
    (
        "Accept",
        "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
    ),
    ("Accept-Language", "en-US,en;q=0.5"),
    ("Upgrade-Insecure-Requests", "1"),
    ("Sec-Fetch-Dest", "document"),
    ("Sec-Fetch-Mode", "navigate"),
    ("Sec-Fetch-Site", "none"),
    ("Sec-Fetch-User", "?1"),
];
const TIMEOUT_SECS: u64 = 15;
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

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

/// GET a top-level document: the Firefox-navigation [`DOCUMENT_HEADERS`]
/// layered *under* the caller's `headers`, so a caller `-H` for any of those
/// names replaces the default rather than duplicating it (same precedence as
/// [`user_agent`]). Subresource subfetches (stylesheets, `fetch`/XHR, external
/// `<script>`) call [`fetch`] directly and do not carry the navigation set.
pub fn fetch_document(
    url: &str,
    headers: &[(String, String)],
) -> Result<FetchResult, FetchError> {
    let mut effective = headers.to_vec();
    for (name, value) in DOCUMENT_HEADERS {
        if !headers.iter().any(|(n, _)| n.eq_ignore_ascii_case(name)) {
            effective.push(((*name).to_string(), (*value).to_string()));
        }
    }
    fetch(url, &effective)
}

/// GET `url` with the caller's `headers` attached. A caller-supplied
/// `User-Agent` replaces the default. `Authorization` is never forwarded
/// across redirects (ureq's default). `file://` reads ignore `headers` —
/// the CLI rejects that combination as a usage error before we get here.
pub fn fetch(url: &str, headers: &[(String, String)]) -> Result<FetchResult, FetchError> {
    let parsed = validate_url(url)?;
    if parsed.scheme() == "file" {
        return fetch_file(&parsed);
    }
    // The connector chain opens TCP, then wraps HTTPS in a Firefox-shaped TLS
    // handshake (`firefox_tls`). ureq keeps HTTP/1.1, redirects, decompression,
    // timeouts and the error taxonomy; only the ClientHello changes.
    let config = Agent::config_builder()
        .user_agent(user_agent(headers))
        .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
        .http_status_as_error(false)
        .build();
    let connector = ()
        .chain(TcpConnector::default())
        .chain(firefox_tls::FirefoxTlsConnector::default());
    let agent = Agent::with_parts(config, connector, DefaultResolver::default());

    let mut request = agent.get(url);
    for (n, v) in headers.iter().filter(|(n, _)| !n.eq_ignore_ascii_case("user-agent")) {
        request = request.header(n.as_str(), v.as_str());
    }
    let mut response = request.call().map_err(map_ureq_error)?;
    let final_url = response.get_uri().to_string();
    let status = response.status().as_u16();

    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(n, v)| {
            (
                n.as_str().to_string(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
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
fn fetch_file(url: &url::Url) -> Result<FetchResult, FetchError> {
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

fn validate_url(url: &str) -> Result<url::Url, FetchError> {
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

pub(crate) fn extract_charset_from_content_type(ct: &str) -> Option<String> {
    let lower = ct.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let after = &ct[idx + "charset=".len()..];
    let end = after
        .find(|c: char| c == ';' || c.is_whitespace())
        .unwrap_or(after.len());
    let trimmed = after[..end].trim_matches(['"', '\'']);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(crate) fn decode_body(bytes: &[u8], content_type: Option<&str>) -> (String, String) {
    let declared = content_type.and_then(extract_charset_from_content_type);
    let sniffed = sniff_meta_charset(bytes);
    let chosen = declared.or(sniffed).unwrap_or_else(|| "utf-8".to_string());
    let encoding =
        encoding_rs::Encoding::for_label(chosen.as_bytes()).unwrap_or(encoding_rs::UTF_8);
    let (decoded, used, _had_errors) = encoding.decode(bytes);
    (decoded.into_owned(), used.name().to_ascii_lowercase())
}

fn sniff_meta_charset(bytes: &[u8]) -> Option<String> {
    let head: &[u8] = if bytes.len() > 1024 {
        &bytes[..1024]
    } else {
        bytes
    };
    let head_str = String::from_utf8_lossy(head);
    let lower = head_str.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let after = head_str[idx + "charset=".len()..].trim_start_matches(['"', '\'']);
    let end = after
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .unwrap_or(after.len());
    if end == 0 {
        None
    } else {
        Some(after[..end].to_string())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod file_tests;

#[cfg(test)]
mod header_tests;
