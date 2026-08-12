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
/// them all, so — like every browser — it renders the element's own replaced
/// box and paints none of what is inside it. Not a "no source yet" or "load
/// failed" state: a broken `<source>` yields an empty player, not the prose.
///
/// The pair's second field is the fact a single boolean could not carry
/// (`bl-e79a`): whether the **accessibility tree** still exposes that markup.
/// It does for `<canvas>` — HTML makes canvas fallback content the element's
/// accessible sub-tree, so a browser paints none of it and exposes all of it —
/// and it does not for the rest, whose subtree the UA replaces with its own
/// controls or a nested browsing context. Measured in Chrome 139 headless at
/// 1280×720 (`bl-e79a`): `<canvas>x</canvas>` yields zero client rects, no
/// `innerText`, and a live `StaticText "x"` in `Accessibility.getFullAXTree`;
/// `<video>`/`<audio>`/`<iframe>` yield zero rects, no `innerText`, and no AX
/// node at all.
///
/// This is a *content model* fact, not a CSS one, so unlike `[hidden]` it holds
/// in every recipe and needs no cascade to see. Its readers are therefore both
/// kinds: the recipe-independent subtree skip in `views::text`, and
/// [`crate::dom::Document::unpainted`] / [`crate::dom::Document::concealed`],
/// the two structural queries that route it into layout, the cascade, `bboxes`,
/// the needs walk, and — through [`crate::dom::Document::ax_children`], the one
/// accessor both AX walks descend through (`bl-0aaf`) — `--out ax`.
///
/// Not in this set, each on measured evidence rather than analogy (`bl-e79a`):
///
/// - **`<object>`** — Chrome paints its fallback exactly when the resource
///   does not become the element's box, which turns on the fetch result, the
///   sniffed MIME type, and plugin support: `data` that 404s, a `data` that
///   loads under an unsupported `type`, and a bare `<object>` with neither
///   attribute all paint the fallback and expose it; `data` that loads, and a
///   `type` with no `data`, paint and expose none of it. frot never fetches
///   `<object data>`, so it has no way to know which, and guessing an outcome
///   would be inventing state. Leaving the fallback rendered keeps the copy
///   that is actually in the document.
/// - **`<canvas>`** is here, but only for the paint half — see the field above.
/// - **`<picture>`** — not fallback at all. Its `<source>` children are void
///   configuration and its `<img>` *is* the rendered element; text sitting
///   directly inside a `<picture>` paints and is exposed exactly like text in a
///   `<span>` (measured), which is what frot already does.
const FALLBACK_TAGS: &[(&str, bool)] = &[
    ("video", false),
    ("audio", false),
    ("iframe", false),
    ("canvas", true),
];

/// `name`'s entry in [`FALLBACK_TAGS`] — `Some(exposed_to_ax)` for a
/// fallback-content element, `None` for every other tag.
fn fallback(name: &str) -> Option<bool> {
    FALLBACK_TAGS
        .iter()
        .find(|(tag, _)| *tag == name)
        .map(|&(_, ax)| ax)
}

/// Whether a UA paints `name`'s children at all — false for every
/// [`FALLBACK_TAGS`] element, true for any other tag (a void element simply has
/// no children to paint).
pub fn renders_children(name: &str) -> bool {
    fallback(name).is_none()
}

/// Whether the accessibility tree exposes `name`'s children — false only for a
/// fallback-content element whose subtree the UA replaces outright. `<canvas>`
/// is the one element that paints no child and exposes every one of them, so
/// this is *not* [`renders_children`] and the two must be asked separately.
pub fn exposes_children(name: &str) -> bool {
    fallback(name).unwrap_or(true)
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
    fn fallback_elements_paint_none_of_their_children() {
        for name in ["video", "audio", "iframe", "canvas"] {
            assert!(!super::renders_children(name), "{name}");
        }
        // `<object>` fallback turns on a load outcome frot never has, and a
        // `<picture>`'s children are not fallback at all (`bl-e79a`).
        for name in ["div", "picture", "object", "source", "img"] {
            assert!(super::renders_children(name), "{name}");
        }
    }

    #[test]
    fn only_canvas_still_exposes_its_unpainted_children() {
        assert!(super::exposes_children("canvas"));
        for name in ["video", "audio", "iframe"] {
            assert!(!super::exposes_children(name), "{name}");
        }
        for name in ["div", "picture", "object"] {
            assert!(super::exposes_children(name), "{name}");
        }
    }
}
