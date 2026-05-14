//! HTTP fetcher: blocking `ureq`-based GET with sane browser-ish defaults.
//!
//! Returns the final URL (post-redirect), status, headers, decoded body, and
//! the charset that was used to decode it. Errors map to the canonical
//! [`crate::envelope::kinds`] taxonomy that the envelope serializer expects.

use std::time::Duration;

use ureq::{Agent, ResponseExt};

use crate::envelope::kinds;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0";
const TIMEOUT_SECS: u64 = 15;
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct FetchResult {
    pub final_url: String,
    pub status: u16,
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

pub fn fetch(url: &str) -> Result<FetchResult, FetchError> {
    validate_url(url)?;
    let agent: Agent = Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
        .http_status_as_error(false)
        .build()
        .into();

    let mut response = agent.get(url).call().map_err(map_ureq_error)?;
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
        status,
        headers,
        body,
        charset,
    })
}

fn validate_url(url: &str) -> Result<(), FetchError> {
    let parsed = url::Url::parse(url)
        .map_err(|e| FetchError::new(kinds::FETCH_URL, format!("invalid URL: {}", e)))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(FetchError::new(
            kinds::FETCH_URL,
            format!("unsupported scheme: {}", parsed.scheme()),
        ));
    }
    Ok(())
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
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
        Error::Tls(_) | Error::Rustls(_) => kinds::FETCH_TLS,
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
    let after = &head_str[idx + "charset=".len()..];
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
