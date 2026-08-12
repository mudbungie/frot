//! `--out text` over the `<select>` subtree (`bl-66ed`).
//!
//! `<option>` was not in `BLOCK_TAGS`, so adjacent options concatenated as
//! inline text: `<option>first</option><option>second</option>` extracted as
//! `firstsecond` — not a missing space but an **invented word**, one that
//! appears nowhere in the document, that no user ever sees, and that a caller
//! grepping for `second` cannot find (VISION principle 5).
//!
//! Oracle: Chrome 139 headless (`--headless=new`) at 1280×720, reading
//! `body.innerText`, `Range.getClientRects()` per text node, `getComputedStyle`
//! and `Accessibility.getFullAXTree`. Measured 2026-08-12:
//!
//! | markup | `innerText` | option rects | computed `display` |
//! |---|---|---|---|
//! | `<select>` (collapsed `size=1`) | all options, one per line | **0** | `inline-block` / options `block` |
//! | `<select size=4>` | all options, one per line | 1 each | options `block` |
//! | `<select multiple>` | all options, one per line | 1 each | options `block` |
//! | `<option>` in a plain `<div>` | one per line | 1 each | `block` |
//! | `<optgroup label=G>` | label **absent** | 0 collapsed, 1 open | `block` |
//! | `<b>` inside one `<option>` | joined, no break | — | `inline` |
//! | `<datalist>` | **nothing** | 0 | `none` |
//! | `<datalist>` + author `display:block` | its options | 1 each | `block` |
//!
//! Two different kinds of fact, so two different homes (`layout.md` §2.4):
//! `block` is the *tag's own* display — it holds in a `<div>` with no select
//! anywhere — so it is `BLOCK_TAGS` and needs no cascade, and every case below
//! is pinned in both recipes. `<datalist>`'s `none` is a UA *declaration* the
//! last row overrides, so it is the cascade's UA-implicit display and holds
//! only under `--css` (`layout.md` §2.3).

use super::tests::{t, t_css};

/// A recipe-independent case: `BLOCK_TAGS` is not cascade-gated, so both
/// recipes state one expectation.
fn both(html: &str, want: &str) {
    assert_eq!(t(html), want, "no --css: {html}");
    assert_eq!(t_css(html), want, "--css: {html}");
}

#[test]
fn adjacent_options_do_not_fuse_into_one_token() {
    both(
        "<select><option>first</option><option>second</option></select>",
        "first\nsecond",
    );
}

#[test]
fn option_breaks_in_every_select_mode() {
    for open in ["size=4", "multiple", "size=1"] {
        both(
            &format!("<select {open}><option>first</option><option>second</option></select>"),
            "first\nsecond",
        );
    }
}

#[test]
fn optgroup_breaks_and_its_label_attribute_is_not_text() {
    both(
        "<select><optgroup label=GroupA><option>first</option><option>second</option></optgroup>\
         <optgroup label=GroupB><option>third</option></optgroup></select>",
        "first\nsecond\nthird",
    );
}

#[test]
fn option_is_block_outside_a_select_too() {
    both(
        "<div><option>bare1</option><option>bare2</option></div>",
        "bare1\nbare2",
    );
}

#[test]
fn inline_markup_inside_one_option_stays_on_its_line() {
    both(
        "<select><option>a<b>bold</b>c</option></select><p>after</p>",
        "aboldc\nafter",
    );
}

#[test]
fn a_select_does_not_swallow_the_text_around_it() {
    both(
        "<p>before</p><select><option>first</option></select><p>after</p>",
        "before\nfirst\nafter",
    );
}

/// The recipe-dependent half: a UA declaration, so `--css` is what sees it.
#[test]
fn datalist_suggestions_are_hidden_only_under_css() {
    let html = "<input list=dl><datalist id=dl><option value=alpha>Alpha label</option>\
                <option value=beta>Beta label</option></datalist>";
    assert_eq!(t_css(html), "");
    assert_eq!(t(html), "Alpha label\nBeta label");
}

#[test]
fn an_author_rule_beats_the_datalist_ua_declaration() {
    assert_eq!(
        t_css(
            "<style>datalist{display:block}</style><input list=dl>\
             <datalist id=dl><option value=alpha>Alpha label</option></datalist>"
        ),
        "Alpha label",
    );
}
