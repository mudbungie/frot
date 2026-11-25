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
    // Chrome: "𝑝\n=\n2". The italic is the UA's `text-transform: math-auto`
    // on a one-letter `<mi>`, which needs the cascade (`bl-9b22`); the line
    // structure holds in both recipes.
    let html = "<math><mrow><mi>p</mi><mo>=</mo><mn>2</mn></mrow></math>";
    assert_eq!(t(html), "p\n=\n2");
    assert_eq!(t_css(html), "\u{1d45d}\n=\n2");
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

/// `bl-fe2c`, Chrome 158 `innerText`: under a cascade the line breaks follow
/// the computed `display`, not the tag; without one, the tag.
#[test]
fn line_breaks_follow_the_computed_display() {
    for (html, want) in [
        ("<button>x<span style='display:block'>sb</span>y</button>", "x\nsb\ny"),
        ("<button>x<span style='display:flex'>fx</span>y</button>", "x\nfx\ny"),
        ("<div>p<span style='display:list-item'>li</span>q</div>", "p\nli\nq"),
        ("<button>x<div style='display:inline'>di</div>y</button>", "xdiy"),
        ("<div>p<span style='display:inline-block'>ib</span>q</div>", "pibq"),
        ("<div>p<span style='display:inline-flex'>if</span>q</div>", "pifq"),
        ("<div>p<li style='display:inline'>il</li>q</div>", "pilq"),
        ("<table><tr><td style='display:inline'>t1</td><td style='display:inline'>t2</td></tr></table>", "t1t2"),
        // A `<p>` stands on its own line whatever its box.
        ("<div>a<p style='display:inline'>ip</p>r</div>", "a\nip\nr"),
    ] {
        assert_eq!(t_css(html), want, "{html}");
    }
    let bare = "<button>x<span style='display:block'>sb</span>y<div style='display:inline'>d</div></button>";
    assert_eq!(t(bare), "xsby\nd");
}

/// `bl-346e`, Chrome 158 `innerText`: a box out of flow, and a block-level
/// `::before`/`::after`, stand on lines of their own.
#[test]
fn an_out_of_flow_box_is_a_line_of_its_own() {
    for (html, want) in [
        (
            "<p>Before<span style='position:absolute'>tip</span>after.</p>",
            "Before\ntip\nafter.",
        ),
        (
            "<p>f1<span style='float:left'>fl</span>f2</p>",
            "f1\nfl\nf2",
        ),
        (
            "<p>r1<span style='position:relative'>rl</span>r2</p>",
            "r1rlr2",
        ),
        ("<p>w1<b style='position:fixed'></b>w2</p>", "w1\nw2"),
        (
            "<style>var::after{content:attr(data-type);position:absolute}</style>\
             <p>Convert <var data-type=ShareData>data</var> now.</p>",
            "Convert data\nShareData\nnow.",
        ),
        (
            "<style>b::before{content:'B';display:block}</style><p>x<b>y</b>z</p>",
            "x\nB\nyz",
        ),
        (
            "<style>b::after{content:'A'}</style><p>x<b>y</b>z</p>",
            "xyAz",
        ),
    ] {
        assert_eq!(t_css(html), want, "{html}");
    }
}

/// Chrome 158 (`bl-01b2`, `svg.html`): an element SVG does not know paints
/// nothing, its subtree included, and SVG text paints only inside `<text>` —
/// not directly in `<svg>` or `<g>`, nor in an unknown element inside `<text>`.
/// A comment is no text either way, and `<foreignObject>`'s children are HTML.
#[test]
fn svg_paints_text_only_in_text_and_nothing_in_an_unknown_element() {
    both(
        "<p>six <svg><foo style=display:block>svgfoo</foo><foo><text>footext</text></foo>\
         rawsvg<!--c--><g>rawg</g><g><text>gtext</text></g><text><foo>intextfoo</foo>\
         <tspan>tsp</tspan></text><feBlend>fe</feBlend>\
         <foreignObject>fo<b>bold</b></foreignObject></svg></p>",
        "six\ngtext\ntsp\nfobold",
    );
}
