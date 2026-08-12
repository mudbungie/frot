//! Media-type disposition: whether a response body is a *document* at all.
//!
//! frot owns exactly one parser, html5ever. Handing it a PNG produces an
//! element tree fabricated out of chunk headers and replacement glyphs, and
//! reporting that as `ok` is the false-`ok` direction VISION principle 5
//! refuses (bl-0c3e). So the top-level media disposition is decided *before*
//! the charset decode and the parse, from the one fact the response already
//! carries: its declared `Content-Type`.
//!
//! The policy, in three rules (ARCHITECTURE.md "Fetch behavior"):
//!
//! 1. **The declaration is the sole authority.** frot never sniffs body bytes
//!    and never reads the URL's extension — it behaves as if every response
//!    carried `X-Content-Type-Options: nosniff`, so a `.png` served as
//!    `text/html` is parsed and PNG magic under `image/png` is refused, both
//!    on the server's word. That is the deliberate sniffing ceiling.
//! 2. **A document is text.** [`is_textual`] passes `text/*`, any `+xml` or
//!    `+json` subtype (so `application/xhtml+xml` and `image/svg+xml` are
//!    markup, as they are), and `xml`/`json`/`javascript`/`ecmascript`.
//!    Everything else declared — `image/png`, `application/pdf`,
//!    `application/octet-stream`, audio, video, fonts — is refused, so a new
//!    binary type is refused by default rather than by an allowlist entry.
//! 3. **A declaration frot cannot read declares nothing.** No `Content-Type`
//!    (a `file://` read, a header-less response) or one with no `type/subtype`
//!    shape leaves the body a document, and it is parsed as one — a malformed
//!    header must not turn a real page into an error.
//!
//! The ceiling this leaves, stated rather than hidden: frot has one parser, so
//! a *textual* non-HTML body (plain text, JSON, XML, SVG) is read by the HTML
//! parser, and markup characters inside it become elements. That is the same
//! text a browser shows, structured more eagerly than a browser would.

/// The longest media-type essence a diagnostic will quote.
const MAX_ESSENCE: usize = 64;

/// The declared media type when it is **not** a document frot can parse —
/// `Some(essence)` refuses the body, `None` lets it through to the parser.
///
/// The returned essence is the bounded, quote-safe `type/subtype` (§1 above):
/// lower-cased, ASCII-graphic characters only, [`MAX_ESSENCE`] at most. The
/// caller puts it in an error message, so nothing wider than a media type —
/// and never a byte of the body — can reach the diagnostic.
pub(crate) fn non_document(content_type: Option<&str>) -> Option<String> {
    let declared = content_type?;
    let head = declared.split(';').next().unwrap_or(declared).trim();
    let (ty, sub) = head.split_once('/')?;
    let (ty, sub) = (
        ty.trim().to_ascii_lowercase(),
        sub.trim().to_ascii_lowercase(),
    );
    if is_textual(&ty, &sub) {
        return None;
    }
    Some(bounded(&format!("{ty}/{sub}")))
}

/// Whether a declared `type/subtype` is text frot reads as a document (§2).
/// Structured-suffix subtypes carry the family's meaning, so `+xml`/`+json`
/// are textual wherever they appear — including `image/svg+xml`, which is
/// markup a browser happens to paint.
fn is_textual(ty: &str, sub: &str) -> bool {
    ty == "text"
        || sub.ends_with("+xml")
        || sub.ends_with("+json")
        || matches!(sub, "xml" | "json" | "javascript" | "ecmascript")
}

/// Bound an essence for quoting in a diagnostic: ASCII-graphic characters
/// only, truncated to [`MAX_ESSENCE`]. A server may send anything; an error
/// message must stay a short, printable, single-line fact.
fn bounded(essence: &str) -> String {
    essence
        .chars()
        .filter(char::is_ascii_graphic)
        .take(MAX_ESSENCE)
        .collect()
}

#[cfg(test)]
mod tests;
