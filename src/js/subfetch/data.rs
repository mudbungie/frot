//! `data:` URL decoding — the non-network half of the §6 subfetch seam
//! (js.md §4.1/§6, RFC 2397 / the WHATWG "data: URL processor").
//!
//! A `data:` URL *is* its own response: the bytes are already in hand, so
//! nothing is dispatched, nothing is timed out, and the §5 network deadline has
//! no purchase on it. What still applies is memory — the decoded body charges
//! the same pooled [`super::SUBFETCH_BYTES`] budget a fetched body does, so a
//! page cannot inline its way past the bound.
//!
//! Shape: `data:[<mediatype>][;base64],<data>`. The payload is percent-decoded
//! (the URL parser percent-encodes whatever it must to make the reference a
//! legal URL, so decoding is not optional), then forgiving-base64 decoded when
//! the media type ends in `;base64`, then decoded to text through the *same*
//! [`decode_body`] charset path an HTTP body takes — one decoder, so a
//! `;charset=` parameter means here exactly what a `Content-Type` header means.

use super::Frozen;
use crate::fetch::decode_body;

/// RFC 2397's default media type when the URL declares none.
const DEFAULT_MIME: &str = "text/plain;charset=US-ASCII";

/// The bytes a `data:` URL carries, or `None` for any other scheme — the one
/// test for "this resource needs no network", read by both [`super::Subfetch::get`]
/// and the preload warm (a data URL has no latency to hide, so warming it is
/// meaningless).
pub(super) fn payload(url: &str) -> Option<&str> {
    url.strip_prefix("data:")
}

/// Decode a `data:` URL [`payload`] into a frozen response named by `url`.
/// Modelled as a 200 with the declared media type as its `Content-Type`, which
/// is what a browser exposes for a data URL response; a payload frot cannot
/// decode is a plain failure, surfaced through the same §10 channel as a
/// transport error.
pub(super) fn decode(url: &str, payload: &str) -> Result<Frozen, String> {
    let (meta, data) = payload
        .split_once(',')
        .ok_or_else(|| format!("malformed data: URL (no comma): {}", brief(url)))?;
    let (mime, is_base64) = media_type(meta.trim());
    let bytes = percent_encoding::percent_decode_str(data).collect::<Vec<u8>>();
    let bytes = if is_base64 {
        base64(&bytes).ok_or_else(|| format!("malformed base64 in data: URL: {}", brief(url)))?
    } else {
        bytes
    };
    let (body, _charset) = decode_body(&bytes, Some(mime));
    Ok(Frozen {
        ok: true,
        status: 200,
        url: url.to_string(),
        body,
        headers: vec![("content-type".to_string(), mime.to_string())],
    })
}

/// Split a data URL's metadata into its media type and the base64 flag. An
/// absent type means RFC 2397's [`DEFAULT_MIME`]; the `;base64` suffix is
/// matched case-insensitively and is not part of the type.
fn media_type(meta: &str) -> (&str, bool) {
    let base64 = meta.to_ascii_lowercase().ends_with(";base64");
    let mime = if base64 {
        &meta[..meta.len() - ";base64".len()]
    } else {
        meta
    };
    if mime.is_empty() {
        (DEFAULT_MIME, base64)
    } else {
        (mime, base64)
    }
}

/// Forgiving-base64 decode: ASCII whitespace and `=` padding are ignored, any
/// other non-alphabet byte fails, and a trailing group holding fewer than 8 bits
/// (a lone sextet) fails — the payload is truncated, not merely padded.
fn base64(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() / 4 * 3);
    let mut acc: u32 = 0;
    let mut bits: u32 = 0;
    for &c in input {
        if c.is_ascii_whitespace() || c == b'=' {
            continue;
        }
        acc = ((acc << 6) | u32::from(sextet(c)?)) & 0xffff;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    (bits < 6).then_some(out)
}

/// One standard-alphabet base64 digit's six bits, or `None` if it is not one.
fn sextet(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// A data URL's head, for an error message: the payload *is* the URL, so naming
/// the whole of it would copy a bundle into the diagnostic. The §10 sink clamps
/// too; this keeps the failure legible at the source.
fn brief(url: &str) -> &str {
    match url.char_indices().nth(60) {
        Some((i, _)) => &url[..i],
        None => url,
    }
}

#[cfg(test)]
mod tests;
