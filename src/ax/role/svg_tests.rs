//! Every row is a Chrome 158 measurement (`Accessibility.getFullAXTree`,
//! `--headless=new`, `bl-07bd`).

use crate::ax::role;
use crate::dom::Document;

fn r(html: &str, tag: &str) -> Option<&'static str> {
    let d = Document::parse(html);
    role(&d, d.find_by_tag(tag)[0])
}

#[test]
fn a_graphic_with_nothing_exposed_inside_is_one_picture() {
    for html in [
        "<svg><rect/></svg>",
        "<svg><title>direct</title><rect/></svg>",
        "<svg aria-label=AL></svg>",
        "<svg><desc>d</desc><rect/></svg>",
        "<svg><g><rect/></g></svg>",
        "<svg><text></text></svg>",
        "<svg><text>  </text></svg>",
        "<svg>raw</svg>",
        "<svg><g>raw</g></svg>",
        "<svg><g aria-hidden=true><text>ah</text></g></svg>",
        "<svg><svg><rect/></svg></svg>",
        "<svg><g role=presentation><title>pt</title></g></svg>",
    ] {
        assert_eq!(r(html, "svg"), Some("img"), "{html}");
    }
}

#[test]
fn a_graphic_with_exposed_content_is_a_document() {
    for html in [
        "<svg><title>T2</title><text>words</text></svg>",
        "<svg><text><tspan>ts</tspan></text></svg>",
        "<svg><g><title>nested</title><rect/></g></svg>",
        "<svg><g><g><title>gg</title></g></g></svg>",
        "<svg><rect><title>rt</title></rect></svg>",
        "<svg><a href=q><text>la</text></a></svg>",
        "<svg><a href=q></a></svg>",
        "<svg><g role=img aria-label=GI><rect/></g></svg>",
        "<svg><svg><title>inner</title></svg></svg>",
    ] {
        assert_eq!(r(html, "svg"), Some("graphics-document"), "{html}");
    }
}

#[test]
fn a_part_has_a_role_only_when_described() {
    for (html, tag, want) in [
        ("<svg><g><title>t</title></g></svg>", "g", Some("group")),
        ("<svg><g><title></title></g></svg>", "g", Some("group")),
        ("<svg><g><desc>d</desc></g></svg>", "g", Some("group")),
        ("<svg><g aria-label=GL></g></svg>", "g", Some("group")),
        ("<svg><g title=attr></g></svg>", "g", Some("group")),
        ("<svg><g><rect/></g></svg>", "g", None),
        (
            "<svg><rect><title>t</title></rect></svg>",
            "rect",
            Some("graphics-symbol"),
        ),
        (
            "<svg><rect><desc>d</desc></rect></svg>",
            "rect",
            Some("graphics-symbol"),
        ),
        (
            "<svg><rect aria-label=RL></rect></svg>",
            "rect",
            Some("graphics-symbol"),
        ),
        ("<svg><rect/></svg>", "rect", None),
        (
            "<svg><circle><title>t</title></circle></svg>",
            "circle",
            Some("graphics-symbol"),
        ),
        (
            "<svg><ellipse><title>t</title></ellipse></svg>",
            "ellipse",
            Some("graphics-symbol"),
        ),
        (
            "<svg><line><title>t</title></line></svg>",
            "line",
            Some("graphics-symbol"),
        ),
        (
            "<svg><path><title>t</title></path></svg>",
            "path",
            Some("graphics-symbol"),
        ),
        (
            "<svg><polygon><title>t</title></polygon></svg>",
            "polygon",
            Some("graphics-symbol"),
        ),
        (
            "<svg><polyline><title>t</title></polyline></svg>",
            "polyline",
            Some("graphics-symbol"),
        ),
        (
            "<svg><use href=#x><title>t</title></use></svg>",
            "use",
            Some("graphics-object"),
        ),
    ] {
        assert_eq!(r(html, tag), want, "{html}");
    }
}

#[test]
fn a_nested_graphic_is_a_group_when_described_and_nothing_otherwise() {
    let d = Document::parse("<svg><svg><title>inner</title></svg><svg><rect/></svg></svg>");
    let svgs = d.find_by_tag("svg");
    assert_eq!(role(&d, svgs[1]), Some("group"));
    assert_eq!(role(&d, svgs[2]), None);
}

/// Chrome 158 (`bl-1c54`): with a cascade, what an `<svg>` exposes is what it
/// shows. The plain [`role`] still answers from the markup.
#[test]
fn a_graphic_whose_content_is_css_hidden_is_one_picture() {
    let under = |html: &str| {
        let doc = Document::parse(html);
        let styles = crate::css::compute(&doc);
        crate::ax::role::role_under(&doc, doc.find_by_tag("svg")[0], Some(&styles))
    };
    for inner in [
        "<text style='display:none'>x</text>",
        "<text style='visibility:hidden'>x</text>",
        "<g><text style='display:none'>x</text></g>",
        "<g style='visibility:hidden'><text>x</text></g>",
        "<a href=q style='display:none'><text>la</text></a>",
        "<foreignObject style='display:none'><p>fo</p></foreignObject>",
    ] {
        let html = format!("<svg aria-label=s>{inner}</svg>");
        assert_eq!(under(&html), Some("img"), "{html}");
        assert_eq!(r(&html, "svg"), Some("graphics-document"), "{html}");
    }
    for inner in [
        "<text>x</text>",
        "<text style='display:none'>x</text><text>y</text>",
        "<text style='display:none'>x</text><circle><title>c</title></circle>",
        "<text style='visibility:hidden'>a<tspan style='visibility:visible'>b</tspan></text>",
    ] {
        let html = format!("<svg aria-label=s>{inner}</svg>");
        assert_eq!(under(&html), Some("graphics-document"), "{html}");
    }
}

/// Chrome 158 (`bl-1c54`): a described `<image>` is an `img` inside its
/// graphic; undescribed it has no node and the graphic is one picture.
#[test]
fn a_described_image_is_an_img_part() {
    let html = "<svg><image><title>it</title></image></svg>";
    assert_eq!(r(html, "image"), Some("img"));
    assert_eq!(r(html, "svg"), Some("graphics-document"));
    assert_eq!(
        r("<svg><image aria-label=L></image></svg>", "image"),
        Some("img")
    );
    let bare = "<svg><image></image></svg>";
    assert_eq!(r(bare, "image"), None);
    assert_eq!(r(bare, "svg"), Some("img"));
}

/// `bl-b3db`, Chrome 158: an SVG `<a>` with neither `href` nor `xlink:href`
/// is a `group` when described, else no node; `xlink:href` alone makes a link
/// inside an `<svg>` and nothing in HTML.
#[test]
fn an_svg_anchor_without_href_is_a_described_part() {
    for (html, want) in [
        (
            "<svg><a><title>t</title><text>la</text></a></svg>",
            Some("group"),
        ),
        (
            "<svg><a aria-label=A><text>lb</text></a></svg>",
            Some("group"),
        ),
        (
            "<svg><a><desc>d</desc><text>lc</text></a></svg>",
            Some("group"),
        ),
        ("<svg><a title=T><text>le</text></a></svg>", Some("group")),
        ("<svg><a><text>ld</text></a></svg>", Some("generic")),
        (
            "<svg><a xlink:href=#x><text>xl</text></a></svg>",
            Some("link"),
        ),
        ("<svg><a href=#x><text>hl</text></a></svg>", Some("link")),
        ("<a xlink:href=#x>html</a>", Some("generic")),
    ] {
        assert_eq!(r(html, "a"), want, "{html}");
    }
}
