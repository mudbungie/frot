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
/// (`line_count * LINE_HEIGHT`, `0` when no rendered word exists). `block_y` is
/// the block's content-box top in viewport coords; fragments are placed at
/// absolute y so their unions are viewport-relative like every other box.
pub(super) fn flow(
    doc: &Document,
    styles: &Styles,
    block_id: NodeId,
    block_y: i32,
    width_px: i32,
    boxes: &mut [Option<Rect>],
) -> i32 {
    let mut words = Vec::new();
    let mut ancestors = Vec::new();
    for &c in &doc.node(block_id).children {
        collect(doc, styles, c, &mut ancestors, &mut words);
    }
    let (rects, line_count) = break_lines(&words, block_y, width_px);
    fill_unions(&words, &rects, boxes);
    line_count * LINE_HEIGHT
}

/// The max-content inline width of `id`'s subtree: every rendered word on one
/// (unwrapped) line, with a single `GLYPH_ADVANCE` gap between adjacent words.
/// `0` when the subtree renders no word. A crude intrinsic size for a flex row
/// item (`layout.md` §6: structural estimate, not exact item sizing); reuses the
/// same [`collect`] tokenizer and metrics as [`flow`] (single source of truth).
pub(super) fn max_content_width(doc: &Document, styles: &Styles, id: NodeId) -> i32 {
    let mut words = Vec::new();
    let mut ancestors = Vec::new();
    for &c in &doc.node(id).children {
        collect(doc, styles, c, &mut ancestors, &mut words);
    }
    let glyphs: i32 = words.iter().map(|w| w.glyphs).sum();
    let gaps = words.len().saturating_sub(1) as i32;
    glyphs * GLYPH_ADVANCE + gaps * GLYPH_ADVANCE
}

/// Walk `id` in source order, appending a [`Word`] per whitespace-separated run
/// of its rendered text. `ancestors` tracks the enclosing inline-element stack; a
/// `display:none`/non-rendered element and its whole subtree are skipped, a text
/// node is tokenized, any other node kind contributes nothing.
fn collect(
    doc: &Document,
    styles: &Styles,
    id: NodeId,
    ancestors: &mut Vec<NodeId>,
    words: &mut Vec<Word>,
) {
    match &doc.node(id).kind {
        NodeKind::Text(t) => {
            for w in t.split_whitespace() {
                words.push(Word {
                    glyphs: w.chars().count() as i32,
                    ancestors: ancestors.clone(),
                });
            }
        }
        NodeKind::Element(_) => {
            if !is_rendered_element(doc, id, styles) {
                return;
            }
            ancestors.push(id);
            for &c in &doc.node(id).children {
                collect(doc, styles, c, ancestors, words);
            }
            ancestors.pop();
        }
        _ => {}
    }
}

/// Greedy word-boundary line-breaking: place each word at the running x-cursor,
/// wrapping to a new line when it would overflow `width_px` on a non-empty line
/// (a lone word wider than `width_px` takes its own line and overflows). Returns
/// each word's fragment rect (parallel to `words`) and the resulting line count.
fn break_lines(words: &[Word], block_y: i32, width_px: i32) -> (Vec<Rect>, i32) {
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
            x,
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
mod tests;
