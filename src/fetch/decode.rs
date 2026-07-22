//! Body decoding: inflate the `Content-Encoding` then pick a character encoding.
//!
//! frot advertises and decodes only `gzip` and `br` (`Accept-Encoding: gzip,
//! br`); `deflate`/`zstd` are a declared residual (identity.md §14), so a body
//! in any other encoding is passed through untouched rather than mis-decoded.
//! The declared `Content-Type` charset then wins; otherwise sniff a `<meta>`
//! charset out of the first 1 KiB; otherwise UTF-8. Shared by both transports
//! (HTTP responses and `file://` reads), so a local file and a header-less
//! HTTP response decode identically.

use std::io::Read;

use super::{header_value, FetchError, MAX_BODY_BYTES};
use crate::envelope::kinds;

/// The `Accept-Encoding` frot advertises — exactly the decoders [`inflate`]
/// implements (I2, identity.md §14). The single source both the request
/// derivation and the transport fallback read, so what frot advertises can never
/// drift from what it can inflate; `deflate`/`zstd` are a declared residual.
pub(crate) const ACCEPT_ENCODING: &str = "gzip, br";

/// Inflate `body` per its `Content-Encoding`. gzip and br are decoded (capped at
/// [`MAX_BODY_BYTES`] to bound a decompression bomb); everything else — identity,
/// absent, or an encoding frot never advertised — passes through unchanged.
pub(crate) fn inflate(headers: &[(String, String)], body: Vec<u8>) -> Result<Vec<u8>, FetchError> {
    match header_value(headers, "content-encoding")
        .map(|e| e.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("gzip") => cap(flate2::read::MultiGzDecoder::new(&body[..]), "gzip"),
        Some("br") => cap(brotli::Decompressor::new(&body[..], 4096), "br"),
        _ => Ok(body),
    }
}

/// Read a decoder to end, capped at [`MAX_BODY_BYTES`]; a decode failure is a
/// body error tagged with the encoding name.
fn cap(reader: impl Read, enc: &str) -> Result<Vec<u8>, FetchError> {
    let mut out = Vec::new();
    reader
        .take(MAX_BODY_BYTES)
        .read_to_end(&mut out)
        .map_err(|e| FetchError::new(kinds::FETCH_BODY, format!("{enc}: {e}")))?;
    Ok(out)
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
