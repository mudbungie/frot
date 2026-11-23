//! What a tag says for itself inside someone else's name, and whether it is
//! spaced off: every case is a Chrome 158 measurement (`bl-3fc9`), the name
//! being that of the `<h2>` around it.

use crate::ax::accessible_name;
use crate::dom::Document;

fn name(inner: &str, css: bool) -> String {
    let doc = Document::parse(&format!("<h2>{inner}</h2>"));
    let styles = css.then(|| crate::css::compute(&doc));
    accessible_name(&doc, doc.find_by_tag("h2")[0], styles.as_ref()).unwrap_or_default()
}

#[test]
fn a_control_is_spaced_off_whatever_its_box() {
    for (inner, want) in [
        ("v<button>inner</button>w", "v inner w"),
        ("a<button></button>c", "a c"),
        ("a<button role=link>bl</button>c", "a bl c"),
        ("a<button style='display:inline'>bi</button>c", "a bi c"),
        ("a<span role=button>X</span>c", "a X c"),
        ("a<a href=x role=button>lb</a>c", "a lb c"),
        (
            "a<span role=tab>X</span>c<span role=textbox>Y</span>d",
            "a X c Y d",
        ),
        // Not controls.
        ("a<span role=link>X</span>c", "aXc"),
        ("a<a href=x>lk</a>c<span role=tooltip>T</span>d", "alkcTd"),
        ("a<span>plain</span>c", "aplainc"),
    ] {
        assert_eq!(name(inner, true), want, "{inner}");
        assert_eq!(name(inner, false), want, "{inner}");
    }
}

#[test]
fn an_image_is_a_word_unless_it_is_presentational() {
    for (inner, want) in [
        ("a<img>c", "a c"),
        ("a<img alt=x>c", "a x c"),
        ("a<img alt=''>c", "ac"),
        ("a<img role=presentation alt=x>c<img role=none>d", "acd"),
        ("a<img alt='' title=T>c", "a c"),
        ("a<img alt='' aria-label=L>c", "a L c"),
    ] {
        assert_eq!(name(inner, true), want, "{inner}");
    }
}

#[test]
fn ruby_annotations_titles_and_mathml_say_nothing() {
    assert_eq!(
        name("a<ruby>rb<rp>(</rp><rt>rt</rt><rp>)</rp></ruby>c", true),
        "arbc"
    );
    assert_eq!(name("a<title>ttl</title>c", true), "ac");
    assert_eq!(name("a<math><mi>x</mi></math>c", true), "a c");
    // An `<area>` belongs to the image that uses its map, not to the text
    // around the `<map>`.
    assert_eq!(name("a<map><area alt=ar href=x></map>c", true), "ac");
}

#[test]
fn a_gauge_says_its_clamped_value() {
    for (inner, want) in [
        ("a<meter value=.5>m</meter>c", "a 0.5 c"),
        ("a<meter value=5 max=10>m</meter>c", "a 5 c"),
        ("a<meter value=1.50>m</meter>c", "a 1 c"),
        ("a<meter>m</meter>c<meter value=x>m</meter>d", "a 0 c 0 d"),
        ("a<meter min=2 max=8 value=1>m</meter>c", "a 2 c"),
        ("a<meter min=2 max=1 value=5>m</meter>c", "a 2 c"),
        ("a<progress value=5 max=10>p</progress>c", "a 5 c"),
        ("a<progress value=0.25>p</progress>c", "a 0.25 c"),
        ("a<progress min=3 value=1>p</progress>c", "a 1 c"),
        ("a<progress>p</progress>c", "a c"),
    ] {
        assert_eq!(name(inner, true), want, "{inner}");
    }
}

#[test]
fn a_frame_says_its_title_and_a_canvas_its_fallback() {
    assert_eq!(
        name("a<iframe></iframe>c<iframe title=IT></iframe>d", true),
        "a c IT d"
    );
    assert_eq!(
        name("a<canvas>cv</canvas>c<canvas></canvas>d", true),
        "a cv cd"
    );
}

/// The UA's quote marks are generated content, so they exist under a cascade
/// only, and an author `::before` replaces the opening one.
#[test]
fn a_q_is_quoted_under_a_cascade() {
    assert_eq!(name("a<q>quote</q>c", true), "a\u{201c}quote\u{201d}c");
    let nested = "a<q>o<q>in<q>3</q></q>o</q>c";
    let want = "a\u{201c}o\u{2018}in\u{201c}3\u{201d}\u{2019}o\u{201d}c";
    assert_eq!(name(nested, true), want);
    assert_eq!(name("a<q>quote</q>c", false), "aquotec");
    let authored = "<style>q::before{content:'<'}</style>a<q>x</q>c";
    assert_eq!(name(authored, true), "a<x\u{201d}c");
}
