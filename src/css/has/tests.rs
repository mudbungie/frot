use crate::css::{query_all, UnsupportedSelector};
use crate::dom::{Document, NodeKind};

/// The ids `selector` matches in `html`, in tree order.
fn ids(html: &str, selector: &str) -> String {
    let doc = Document::parse(html);
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    hits.iter().map(id).collect::<Vec<_>>().join(" ")
}

// Measured in Chrome 158 over this page (`bl-2a85`).
const PAGE: &str = "<html id=H><body id=B><div id=a1><p id=a2><b id=a3>x</b></p>\
    <i id=a4>y</i></div><div id=a5><i id=a6></i></div><div id=a7></div>";

#[test]
fn each_combinator_reaches_from_the_anchor() {
    assert_eq!(ids(PAGE, ":has(b)"), "H B a1 a2");
    assert_eq!(ids(PAGE, ":has(> b)"), "a2");
    assert_eq!(ids(PAGE, ":has(+ i)"), "a2");
    assert_eq!(ids(PAGE, ":has(~ div)"), "a1 a5");
    assert_eq!(ids(PAGE, "div:has(> i)"), "a1 a5");
}

#[test]
fn the_leftmost_compound_is_anchored() {
    assert_eq!(ids(PAGE, ":has(p b)"), "H B a1");
    assert_eq!(ids(PAGE, ":has(> p > b)"), "a1");
    assert_eq!(ids(PAGE, ":has(+ div i)"), "a1");
    assert_eq!(ids(PAGE, ":has(~ div > i)"), "a1");
}

#[test]
fn it_nests_in_the_logical_pseudo_classes() {
    assert_eq!(ids(PAGE, ":is(:has(b))"), "H B a1 a2");
    assert_eq!(
        ids(
            PAGE,
            "div:not(:has(b)), p:not(:has(b)), b:not(:has(b)), i:not(:has(b))"
        ),
        "a3 a4 a5 a6 a7"
    );
    assert_eq!(ids(PAGE, ":has(b, nope)"), "H B a1 a2");
}

#[test]
fn an_invalid_argument_voids_it() {
    let doc = Document::parse(PAGE);
    // `:has(:is(:has(b)))` is valid in Chrome, whose forgiving `:is()` drops
    // the inner `:has()` and so matches nothing; frot parses no list
    // forgivingly and voids it — the same matches, but a query throws.
    for bad in [
        ":has()",
        ":has(::before)",
        ":has(:has(b))",
        ":has(:is(:has(b)))",
        ":has(:where(:has(b)))",
        ":has(:not(:has(b)))",
        ":has(b, ..)",
    ] {
        let got = query_all(&doc, Document::ROOT, bad);
        assert_eq!(got, Err(UnsupportedSelector), "{bad}");
    }
    let nested = ":has(:nth-child(1 of :has(b)))";
    assert_eq!(
        query_all(&doc, Document::ROOT, nested),
        Err(UnsupportedSelector)
    );
}

#[test]
fn it_weighs_as_its_most_specific_argument() {
    use crate::css::selector::Specificity;
    let sel = crate::css::selparse::parse_selector(":has(> #i, .c)").unwrap();
    assert_eq!(sel.specificity(), Specificity(1, 0, 0));
}
