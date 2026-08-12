//! `--out text` over foreign content: SVG and MathML (`bl-c0a4`).
//!
//! `SKIP_TAGS` carried `svg` and `math` from the start, which made `text` the
//! one frot view that disagreed with the other two *and* with the browser:
//! `ax` already emitted `link "SVG_LINK_TEXT"` and `bboxes` already emitted a
//! sized `text` entry carrying the same string. Omitting text a user sees
//! presents a degraded impression as a complete one (VISION principle 5).
//!
//! Oracle: Chrome 139 headless (`--headless=new`) at 1280×720, reading
//! `body.innerText`, `Range.getClientRects()` per text node, and
//! `getComputedStyle`. Measured 2026-08-12:
//!
//! | markup | `innerText` | rects | computed `display` |
//! |---|---|---|---|
//! | `<text>SVG_LINK_TEXT</text>` | yes | 1 | `block` |
//! | `<mi>MATH_MI</mi>` | yes | 1 | `block math` |
//! | `<tspan>` inside `<text>` | joined, no break | 1 | `inline` |
//! | `<foreignObject><p>` | yes, as HTML | 1 | `block` |
//! | SVG `<title>`, `<desc>`, `<metadata>` | **no** | 0 | — |
//! | MathML `<annotation>` | **no** | 0 | — |
//! | `<style>`/`<script>` inside `<svg>` | **no** | 0 | — |
//! | HTML `<head><title>` | **no** | 0 | — |
//!
//! `<svg>` and `<math>` themselves compute `inline`, so they break no line;
//! that is why the block set gained the elements *inside* them rather than the
//! roots. All of it is recipe-independent — none of these facts needs a
//! cascade — so every case below is pinned with and without `--css`.

use super::tests::{t, t_css};

/// Both recipes agree, so every case states one expectation.
fn both(html: &str, want: &str) {
    assert_eq!(t(html), want, "no --css: {html}");
    assert_eq!(t_css(html), want, "--css: {html}");
}

#[test]
fn svg_link_text_is_page_text() {
    both(
        "<div><svg width=60 height=40><a href='/x'><text x=0 y=20>SVG_LINK_TEXT</text></a>\
         </svg></div>",
        "SVG_LINK_TEXT",
    );
}

#[test]
fn mathml_token_text_is_page_text() {
    both("<div><math><mi>MATH_MI</mi></math></div>", "MATH_MI");
}

#[test]
fn svg_source_and_data_still_do_not_surface() {
    // The narrow half of the fix: the blanket skip went, so every element the
    // browser gives zero rects needs its own entry. `<title>` covers the SVG
    // tooltip and the HTML document title with one rule, both measured absent.
    both(
        "<div>seen</div><svg width=80 height=40><title>SVG_TITLE</title>\
         <desc>SVG_DESC</desc><metadata>SVG_METADATA</metadata>\
         <style>.q{fill:red}</style><script>var q=1;</script>\
         <text x=0 y=20>SVG_TEXT</text></svg>",
        "seen\nSVG_TEXT",
    );
    both(
        "<div><math><semantics><mrow><mi>SEM</mi></mrow>\
         <annotation encoding='text'>ANNOT</annotation></semantics></math></div>",
        "SEM",
    );
    both(
        "<html><head><title>HEAD_TITLE</title></head><body><p>body</p></body></html>",
        "body",
    );
}

#[test]
fn each_svg_text_element_is_its_own_line() {
    // Chrome: `<text>Jan</text><text>Feb</text>` is "Jan\nFeb" — a chart's
    // labels are separate lines, not one run. The break is at the `<text>`
    // element, not at every text node, which `<tspan>` is what proves.
    both(
        "<svg width=200 height=60><text x=0 y=20>Jan</text><text x=0 y=40>Feb</text></svg>",
        "Jan\nFeb",
    );
    both(
        "<svg width=200 height=30><text x=0 y=20>A<tspan>B</tspan>C</text></svg>",
        "ABC",
    );
}

#[test]
fn each_mathml_token_is_its_own_line() {
    // Chrome: "𝑝\n=\n2" — the italic is a font transform frot does not invent,
    // so it reports the source characters; the line structure is the claim.
    both(
        "<math><mrow><mi>p</mi><mo>=</mo><mn>2</mn></mrow></math>",
        "p\n=\n2",
    );
}

#[test]
fn foreign_object_content_reads_as_ordinary_html() {
    // Nothing was needed for this: `<foreignObject>` holds real HTML, so the
    // inline/block rules already apply once the blanket skip is gone. Note the
    // lowercase tag — the parser case-folds every name into the arena.
    both(
        "<svg width=200 height=60><foreignObject width=200 height=60>\
         <span>x</span><span>y</span><div>z</div></foreignObject></svg>",
        "xy\nz",
    );
}

#[test]
fn svg_text_stays_out_of_a_neighbouring_run() {
    // The whole page shape, so the line breaks are pinned in context rather
    // than one fixture at a time. Chrome gives exactly this, modulo its
    // MathML italic transform.
    both(
        "<div>a<svg width=30 height=20><text x=0 y=10>INLINE_SVG</text></svg>b</div>",
        "a\nINLINE_SVG\nb",
    );
}
