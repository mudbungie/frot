use crate::css::query_all;
use crate::dom::{Document, NodeKind};

/// The ids `selector` matches in `html`, in tree order.
fn ids(html: &str, selector: &str) -> String {
    ids_at(html, "", selector)
}

/// [`ids`] with the page at a URL ending in `fragment`.
fn ids_at(html: &str, fragment: &str, selector: &str) -> String {
    let mut doc = Document::parse(html);
    doc.set_url(&format!("https://example.com/{fragment}"));
    let hits = query_all(&doc, Document::ROOT, selector).unwrap();
    let id = |&n: &u32| match &doc.node(n).kind {
        NodeKind::Element(e) => e.attr("id").unwrap_or("-").to_string(),
        other => panic!("matched a non-element: {other:?}"),
    };
    hits.iter().map(id).collect::<Vec<_>>().join(" ")
}

#[test]
fn nothing_is_focused_without_an_autofocus_candidate() {
    let html = "<input id=a><a id=b href=x>l</a>";
    assert_eq!(ids(html, ":focus, :focus-visible, :focus-within"), "");
    assert_eq!(ids(html, "input:not(:focus)"), "a");
}

#[test]
fn the_first_candidate_in_tree_order_takes_focus() {
    let html = "<div id=w><!-- c --><input id=a autofocus></div><input id=b autofocus>";
    assert_eq!(ids(html, ":focus"), "a");
    assert_eq!(ids(html, ":focus-visible"), "a");
    assert_eq!(ids(html, "[id]:focus-within"), "w a");
    assert_eq!(ids(html, "input:not(:focus)"), "b");
}

#[test]
fn a_candidate_that_cannot_take_focus_is_skipped() {
    for skipped in [
        "<input id=a autofocus disabled>",
        "<fieldset disabled><button id=a autofocus></button></fieldset>",
        "<select id=a autofocus disabled></select>",
        "<input id=a type=hidden autofocus>",
        "<span id=a autofocus>x</span>",
        "<a id=a autofocus>x</a>",
        "<span id=a tabindex=soon autofocus>x</span>",
        "<span id=a contenteditable=false autofocus>x</span>",
        "<input id=a hidden autofocus>",
        "<div hidden><input id=a autofocus></div>",
        "<div inert><input id=a autofocus></div>",
        "<details><summary>s</summary><input id=a autofocus></details>",
    ] {
        let html = format!("{skipped}<input id=b autofocus>");
        assert_eq!(ids(&html, ":focus"), "b", "{skipped}");
    }
}

#[test]
fn every_focusable_kind_is_a_candidate() {
    for taken in [
        "<button id=a autofocus></button>",
        "<select id=a autofocus></select>",
        "<textarea id=a autofocus></textarea>",
        "<a id=a href=x autofocus>x</a>",
        "<span id=a tabindex=' -1 ' autofocus>x</span>",
        "<span id=a contenteditable autofocus>x</span>",
    ] {
        let html = format!("{taken}<input id=b autofocus>");
        assert_eq!(ids(&html, ":focus"), "a", "{taken}");
    }
}

#[test]
fn the_fragment_names_the_target_and_a_target_stops_autofocus() {
    let html = "<a id=t name=n>x</a><p id=u>u</p><input id=f autofocus>";
    assert_eq!(ids_at(html, "#u", ":target"), "u");
    assert_eq!(ids_at(html, "#n", ":target"), "t");
    assert_eq!(ids_at(html, "#u", "p:not(:target), :focus"), "");
    assert_eq!(ids_at(html, "#nothing", ":target, :focus"), "f");
    assert_eq!(ids(html, ":target, :focus"), "f");
}
