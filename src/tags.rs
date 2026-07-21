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

#[cfg(test)]
mod tests {
    use super::is_block;

    #[test]
    fn membership_covers_block_and_inline() {
        assert!(is_block("div"));
        assert!(is_block("li"));
        assert!(!is_block("span"));
    }
}
