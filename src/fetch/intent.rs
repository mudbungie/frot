//! The request's role in the page load — the one seam every per-destination
//! policy (headers, cookies, caching, decoding) derives from.

/// A request's role in the page load (`identity.md` §4.2: destination ∈
/// {document, style, script, empty, image}). The session carries it on every
/// request as the seam [`super::request::derive_headers`] derives
/// per-destination headers and protocol metadata from. It gates caching (a
/// navigation is never cached, nor is an image; every other subresource is a
/// cacheable safe GET) and selects the per-class request metadata
/// (`identity.md` §4.1) — each class differs in `Accept`, Fetch Metadata, and
/// `Priority`.
#[derive(Clone, Copy)]
pub enum Intent {
    /// The top-level document GET — never cached, carries the nav header set.
    Navigation,
    /// An external `<link rel=stylesheet>` (the `--css` gather).
    Style,
    /// An external classic `<script src>`.
    ClassicScript,
    /// An external ES module or an `import` graph edge.
    Module,
    /// A page `fetch()` / `XMLHttpRequest`.
    FetchXhr,
    /// An `<img>` request under `--images` (js.md §7 "Images", `bl-49f1`).
    /// `background` is Firefox's lower `Priority`: the `<img>` was not in the
    /// document as its request started, or it is `loading=lazy`.
    Image { background: bool },
}

impl Intent {
    /// Whether the response is an image: its body is bytes, never text, so it
    /// is decoded byte-for-byte ([`super::decode::binary`]) and kept out of the
    /// session's text cache.
    pub(crate) fn is_image(self) -> bool {
        matches!(self, Intent::Image { .. })
    }
}
