//! Shared HTML tag classifications.
//!
//! Single source of truth for the set of block-level element names. Both the
//! `text` view's block-break logic (`views::text`) and the CSS cascade's
//! UA-implicit `display` step (`css::cascade`) read this set, so the two can
//! never drift. Do not change its membership without accounting for both
//! consumers: it drives `text` view line breaks *and* implicit `display`.

/// Block-level HTML tag names — the authoritative shared set (see module docs).
pub const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "dd",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "html",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "table",
    "tr",
    "td",
    "th",
    "ul",
];

/// Whether `name` is a block-level tag per [`BLOCK_TAGS`].
pub fn is_block(name: &str) -> bool {
    BLOCK_TAGS.contains(&name)
}

/// Tags whose children are **fallback content**: markup HTML defines as shown
/// only by a user agent that does not implement the element. frot implements
/// both, so — like every browser — it renders the media box and never what is
/// inside it. Not a "no source yet" or "playback failed" state: a broken
/// `<source>` yields an empty player, not the prose.
///
/// This is a *content model* fact, not a CSS one, so unlike `[hidden]` it holds
/// in every recipe and needs no cascade to see. Its readers are therefore both
/// kinds: the recipe-independent subtree skips in `views::text` and `ax::tree`,
/// and [`crate::dom::Document::concealed`], which routes it into the cascade
/// (and so into geometry, `bboxes` and the needs walk).
const FALLBACK_TAGS: &[&str] = &["video", "audio"];

/// Whether a UA renders `name`'s children at all — false for the media
/// elements of [`FALLBACK_TAGS`], true for every other tag (a void element
/// simply has no children to render).
pub fn renders_children(name: &str) -> bool {
    !FALLBACK_TAGS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::is_block;

    #[test]
    fn membership_covers_block_and_inline() {
        assert!(is_block("div"));
        assert!(is_block("li"));
        assert!(!is_block("span"));
    }

    #[test]
    fn only_media_elements_withhold_their_children() {
        assert!(!super::renders_children("video"));
        assert!(!super::renders_children("audio"));
        // `<picture>`/`<object>`/`<canvas>` carry fallback of their own kinds;
        // they are deliberately not in this set until measured (`bl-e79a`).
        for name in ["div", "picture", "object", "canvas", "iframe", "img"] {
            assert!(super::renders_children(name), "{name}");
        }
    }
}
