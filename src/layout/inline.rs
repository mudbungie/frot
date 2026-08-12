//! Inline formatting pass (Phase 3 subtask 3.4) — greedy word-wrapping with
//! fixed-advance text metrics.
//!
//! A block box establishing an inline formatting context (a block with no
//! block-level child box; see [`super::layout_block`]) has its content laid out
//! here: rendered text and inline elements are tokenized into whitespace-
//! separated words and packed left-to-right, wrapping at word boundaries against
//! the block width. The pass returns the block's inline content height
//! (`line_count * LINE_HEIGHT`) and fills each inline element's box with the
//! union of its word fragments.
//!
//! **One coordinate space** (`layout.md` §1/§6): line-breaking runs in
//! block-local coordinates, and the containing block's content origin is composed
//! onto each fragment as it is materialized — on `x` exactly as on `y`. A
//! descendant of an indented or flex-placed block therefore keeps its ancestor's
//! origin instead of restarting at `x = 0`.
//!
//! **`::before`/`::after` generated content is inline content** (`layout.md`
//! §6): an element's computed generated strings are tokenized by the same
//! [`tokenize`] as a text child, at the same position a browser puts them —
//! `::before` ahead of the children, `::after` behind — so an icon span with no
//! text still gets a box, and the content that follows it shifts. A pseudo whose
//! `display` is `none` computes no string (`css::computed`) and so contributes
//! nothing here. Generated content in a container that also has block-level
//! children is dropped exactly as a stray text child there is: no anonymous
//! block promotion (`layout.md` §1).
//!
//! **These are structural estimates, not pixel truth** (design §6): no font is
//! loaded, so a word's width is `glyphs * GLYPH_ADVANCE`, an inter-word space
//! advances the same, and every line is `LINE_HEIGHT` tall. The value is
//! relative structure and reading order, not exact widths — a documented
//! approximation. An `inline-block`/`flex` descendant is treated as plain inline
//! text (no atomic sizing); precise inline-block/flex sizing is later work.

use std::collections::HashMap;

use super::{is_rendered_element, Rect, GLYPH_ADVANCE, LINE_HEIGHT};
use crate::css::Styles;
use crate::dom::{Document, NodeId, NodeKind};

/// A collected inline word: its length in glyphs and the stack of inline-element
/// ancestors (outermost first) it sits under, used to union fragment rects.
struct Word {
    glyphs: i32,
    ancestors: Vec<NodeId>,
}

/// Lay out `block_id`'s inline content against `width_px`, filling inline-element
/// fragment-union rects into `boxes` and returning the block's content height
/// (`line_count * LINE_HEIGHT`, `0` when no rendered word exists). `(block_x,
/// block_y)` is the block's content-box origin in viewport coords; both axes are
/// composed onto every fragment, so the unions are viewport-relative like every
/// other box and a descendant never loses its containing block's origin.
pub(super) fn flow(
    doc: &Document,
    styles: &Styles,
    block_id: NodeId,
    block_x: i32,
    block_y: i32,
    width_px: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let words = content_of(doc, styles, block_id);
    let (rects, line_count) = break_lines(&words, block_x, block_y, width_px);
    fill_unions(&words, &rects, boxes);
    line_count * LINE_HEIGHT
}

/// The max-content inline width of `id`'s subtree: every rendered word on one
/// (unwrapped) line, with a single `GLYPH_ADVANCE` gap between adjacent words.
/// `0` when the subtree renders no word. A crude intrinsic size for a flex row
/// item (`layout.md` §6: structural estimate, not exact item sizing); reuses the
/// same [`content_of`] tokenizer and metrics as [`flow`], generated content
/// included (single source of truth).
pub(super) fn max_content_width(doc: &Document, styles: &Styles, id: NodeId) -> i32 {
    let words = content_of(doc, styles, id);
    let glyphs: i32 = words.iter().map(|w| w.glyphs).sum();
    let gaps = words.len().saturating_sub(1) as i32;
    glyphs * GLYPH_ADVANCE + gaps * GLYPH_ADVANCE
}

/// Every [`Word`] element `id` contributes as inline content, in layout order.
/// The enclosing inline-element stack starts empty: `id`'s own box is sized by
/// its block/flex caller, not by the fragment union.
fn content_of(doc: &Document, styles: &Styles, id: NodeId) -> Vec<Word> {
    let mut words = Vec::new();
    contents(doc, styles, id, &mut Vec::new(), &mut words);
    words
}

/// Rendered element `id`'s inline content in layout order: its `::before`
/// generated string, its children, then its `::after` — generated content is
/// tokenized exactly like a text child in that position, so it wraps, advances
/// the line cursor, and unions into `id` and its inline ancestors like any other
/// text (`layout.md` §6).
fn contents(
    doc: &Document,
    styles: &Styles,
    id: NodeId,
    ancestors: &mut Vec<NodeId>,
    words: &mut Vec<Word>,
) {
    tokenize(styles.before(id), ancestors, words);
    for &c in &doc.node(id).children {
        collect(doc, styles, c, ancestors, words);
    }
    tokenize(styles.after(id), ancestors, words);
}

/// Walk `id` in source order, appending a [`Word`] per whitespace-separated run
/// of its rendered text. `ancestors` tracks the enclosing inline-element stack; a
/// `display:none`/non-rendered element and its whole subtree are skipped (its
/// generated content with it), a text node is tokenized, any other node kind
/// contributes nothing.
fn collect(
    doc: &Document,
    styles: &Styles,
    id: NodeId,
    ancestors: &mut Vec<NodeId>,
    words: &mut Vec<Word>,
) {
    match &doc.node(id).kind {
        NodeKind::Text(t) => tokenize(Some(t), ancestors, words),
        NodeKind::Element(_) => {
            if !is_rendered_element(doc, id, styles) {
                return;
            }
            ancestors.push(id);
            contents(doc, styles, id, ancestors, words);
            ancestors.pop();
        }
        _ => {}
    }
}

/// Append one [`Word`] per whitespace-separated run of `text` under the current
/// inline-ancestor stack — the single tokenizer for both DOM text and generated
/// content, so the two are metrically indistinguishable.
fn tokenize(text: Option<&str>, ancestors: &[NodeId], words: &mut Vec<Word>) {
    if let Some(text) = text {
        for w in text.split_whitespace() {
            words.push(Word {
                glyphs: w.chars().count() as i32,
                ancestors: ancestors.to_vec(),
            });
        }
    }
}

/// Greedy word-boundary line-breaking: place each word at the running x-cursor,
/// wrapping to a new line when it would overflow `width_px` on a non-empty line
/// (a lone word wider than `width_px` takes its own line and overflows). Returns
/// each word's fragment rect (parallel to `words`) and the resulting line count.
///
/// The cursor is **line-local** — `x == 0` is "line start", and the wrap test is
/// against the content width — and the containing block's origin `(block_x,
/// block_y)` is composed on only when the fragment is materialized, symmetrically
/// on both axes. Line-breaking therefore never sees viewport coordinates and
/// every emitted rect is in the one coordinate space (`layout.md` §6).
fn break_lines(words: &[Word], block_x: i32, block_y: i32, width_px: i32) -> (Vec<Rect>, i32) {
    let mut rects = Vec::with_capacity(words.len());
    let mut x = 0;
    let mut line = 0;
    for word in words {
        let w = word.glyphs * GLYPH_ADVANCE;
        if x > 0 && x + w > width_px {
            line += 1;
            x = 0;
        }
        rects.push(Rect {
            x: block_x + x,
            y: block_y + line * LINE_HEIGHT,
            w,
            h: LINE_HEIGHT,
        });
        x += w + GLYPH_ADVANCE;
    }
    let line_count = if words.is_empty() { 0 } else { line + 1 };
    (rects, line_count)
}

/// Union each word's fragment into every inline-element ancestor's box, writing
/// the result over the [`Rect::ZERO`] placeholder [`super::walk`] seeded. Inline
/// elements with no rendered word are never touched here, so they keep it.
fn fill_unions(words: &[Word], rects: &[Rect], boxes: &mut [Option<Rect>]) {
    let mut unions: HashMap<NodeId, Rect> = HashMap::new();
    for (word, &rect) in words.iter().zip(rects) {
        for &el in &word.ancestors {
            unions
                .entry(el)
                .and_modify(|u| *u = union(*u, rect))
                .or_insert(rect);
        }
    }
    for (el, rect) in unions {
        boxes[el as usize] = Some(rect);
    }
}

/// The smallest rect covering both `a` and `b` (min near corner, max far edge).
fn union(a: Rect, b: Rect) -> Rect {
    let x = a.x.min(b.x);
    let y = a.y.min(b.y);
    let w = (a.x + a.w).max(b.x + b.w) - x;
    let h = (a.y + a.h).max(b.y + b.h) - y;
    Rect { x, y, w, h }
}

#[cfg(test)]
mod origin_tests;
#[cfg(test)]
mod pseudo_tests;
#[cfg(test)]
mod tests;
