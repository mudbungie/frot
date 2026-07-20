//! Body decoding: pick a character encoding for a fetched byte string.
//!
//! The declared `Content-Type` charset wins; otherwise sniff a `<meta>`
//! charset out of the first 1 KiB; otherwise UTF-8. Shared by both transports
//! (HTTP responses and `file://` reads), so a local file and a header-less
//! HTTP response decode identically.

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
