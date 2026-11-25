//! The source selection against Firefox 153.0esr (`bl-0e4a`): every row is a
//! page loaded on the rig (js.md §7 "Images"), at 1280×720 and device pixel
//! ratio 1, and the one image it requested.

use super::select;
use crate::dom::Document;

/// What the first `<img>` of `html` requests: its URL as written, or `-`.
fn chosen(html: &str) -> String {
    let doc = Document::parse(html);
    let img = doc.find_by_tag("img")[0];
    select(&doc, img).map_or("-".into(), |(u, _)| u)
}

/// Each row: the markup, and the URL Firefox requested (`-`: none).
const MEASURED: &[(&str, &str)] = &[
    ("<img src=src srcset='a1 1x, a2 2x'>", "a1"),
    ("<img src=src srcset='b2 2x'>", "src"),
    ("<img srcset='b2 2x, b3 3x'>", "b2"),
    ("<img srcset='l5 0.5x, l8 0.8x'>", "l8"),
    ("<img srcset='w400 400w, w800 800w, w1600 1600w'>", "w1600"),
    ("<img src=src srcset='w400 400w, w800 800w'>", "w800"),
    ("<img sizes='(max-width: 600px) 100vw, 300px' srcset='w400 400w, w800 800w'>", "w400"),
    ("<img sizes=50vw srcset='w400 400w, w800 800w, w1600 1600w'>", "w800"),
    ("<img sizes='calc(50vw - 40px)' srcset='w400 400w, w800 800w'>", "w800"),
    ("<img sizes=400px srcset='w400 400w, w800 800w'>", "w400"),
    ("<img srcset='d1, d2 1x'>", "d1"),
    ("<img sizes=400px srcset='a 400w, b 400w'>", "a"),
    ("<img srcset='e1 1.5q, e2'>", "e2"),
    ("<img src='' srcset=f>", "f"),
    ("<img sizes=400px srcset='h 400w 300h, g 800w'>", "h"),
    ("<img srcset='h 300h, g 2x'>", "g"),
    ("<img sizes=400px srcset='x 1x, w 800w'>", "x"),
    ("<img sizes='foo, 200px' srcset='w400 400w, w800 800w'>", "w400"),
    ("<img sizes='(min-width: 2000px) 100px' srcset='w400 400w, w1600 1600w'>", "w1600"),
    ("<img sizes=20em srcset='w300 300w, w400 400w'>", "w400"),
    ("<img loading=lazy width=200 sizes=auto srcset='w400 400w, w1600 1600w'>", "w400"),
    ("<img loading=lazy sizes=auto srcset='a 100w, w400 400w, b 1600w'>", "w400"),
    ("<img width=200 sizes='auto, 1000px' srcset='w400 400w, w1600 1600w'>", "w1600"),
    ("<picture><source media='(min-width: 2000px)' srcset=big><source type=image/jxl srcset=jxl>\
      <source type=image/webp srcset=webp><img src=fb></picture>", "webp"),
    ("<picture><source type='IMAGE/PNG; charset=x' srcset=png><img src=fb></picture>", "png"),
    ("<picture><source type='' srcset=empty><img src=fb></picture>", "empty"),
    ("<picture><source type=image/tiff srcset=t><img src=fb></picture>", "fb"),
    ("<picture><source media='(max-width: 100px)' srcset=s><img src=fb srcset='fb2 2x'></picture>", "fb"),
    ("<picture><source srcset='' media=all><source srcset=ok><img src=fb></picture>", "ok"),
    ("<picture><source sizes=300px srcset='w400 400w, w800 800w'><img src=fb></picture>", "w400"),
    ("<picture><img src=fb><source srcset=after></picture>", "fb"),
    ("<picture><source src=s><img src=fb></picture>", "fb"),
    ("<picture><source media='foo bar' srcset=s><img src=fb></picture>", "fb"),
    ("<picture>text<!--c--><span></span><img src=fb></picture>", "fb"),
    ("<img srcset='a,b 1x'>", "a,b"),
    ("<img srcset='  a  ,  b 2x , '>", "a"),
    ("<img srcset='a 2x, b 2.0x, c 0x'>", "a"),
    ("<img srcset='a 1e0x, b 2x'>", "a"),
    ("<img srcset='a 0w, b 2x'>", "b"),
    ("<img srcset='a 1x (foo, bar), b 2x'>", "b"),
    ("<img srcset='a 1x 2x, b 3x'>", "b"),
    ("<img sizes=0px srcset='w400 400w, w800 800w'>", "w400"),
    ("<img sizes='-5px, 300px' srcset='w400 400w, w800 800w'>", "w400"),
    ("<img sizes=50vh srcset='w300 300w, w400 400w'>", "w400"),
    ("<img sizes=300 srcset='w400 400w, w1600 1600w'>", "w1600"),
    ("<img srcset='a 0x, b 0.5x'>", "b"),
    ("<img src=plain>", "plain"),
    ("<img src=''>", "-"),
    ("<img>", "-"),
];

#[test]
fn the_pick_is_the_one_firefox_requested() {
    for (html, want) in MEASURED {
        assert_eq!(chosen(html), *want, "{html}");
    }
}

/// One descriptor alone: whether Firefox requested the candidate at all.
#[test]
fn a_descriptor_is_valid_exactly_when_firefox_takes_it() {
    let valid = [
        "0x", ".5x", "1.0e1x", "-0x", "1e-1x", "1x", "100w", "100w 50h",
    ];
    let invalid = [
        "-1x",
        "+1x",
        "1.x",
        "1X",
        "1x 1x",
        "1.5",
        "0400w",
        "+400w",
        "400.0w",
        "1e2w",
        "100w 0h",
        "100w 50h 1x",
        "100w 100w",
        "0w",
        "400W",
        "50h 100w 2h",
        "1é",
        "1x 100w",
        "x",
        "abcx",
    ];
    for d in valid {
        assert_eq!(chosen(&format!("<img srcset='a {d}'>")), "a", "{d}");
    }
    for d in invalid {
        assert_eq!(chosen(&format!("<img srcset='a {d}'>")), "-", "{d}");
    }
}

/// The density the pick is shown at: `x` as written, `w` over the size, and a
/// zero size an infinite density (measured: natural size 0).
#[test]
fn the_density_comes_with_the_pick() {
    let density = |html: &str| {
        let doc = Document::parse(html);
        select(&doc, doc.find_by_tag("img")[0]).map(|(_, d)| d)
    };
    assert_eq!(density("<img srcset='n 2x'>"), Some(2.0));
    assert_eq!(density("<img sizes=100px srcset='w 300w'>"), Some(3.0));
    assert_eq!(
        density("<img sizes=0 srcset='w 300w'>"),
        Some(f64::INFINITY)
    );
    assert_eq!(density("<img src=p>"), Some(1.0));
    // A node that is not an element has no pick.
    let doc = Document::parse("<p>t</p>");
    let text = doc.node(doc.find_by_tag("p")[0]).children[0];
    assert_eq!(select(&doc, text), None);
}
