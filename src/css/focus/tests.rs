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
        "<details open><summary>s</summary><summary id=a autofocus>t</summary></details>",
        "<summary id=a autofocus>loose</summary>",
        "<video id=a autofocus></video>",
        "<area id=a href=x autofocus>",
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
        "<details><summary id=a autofocus>s</summary></details>",
        "<iframe id=a autofocus></iframe>",
        "<video id=a controls autofocus></video>",
        "<audio id=a controls autofocus></audio>",
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

use crate::css::{focusable, focused_element};
use crate::dom::live::Focus;

#[test]
fn a_script_moves_focus_and_autofocus_is_then_skipped() {
    let mut doc =
        Document::parse("<div id=w><input id=a></div><input id=b autofocus><p id=p>p</p>");
    let (a, b) = (doc.find_by_tag("input")[0], doc.find_by_tag("input")[1]);
    assert_eq!(focused_element(&doc), Some(b));
    doc.set_focus(Focus::On(a));
    let hits = |doc: &Document, s| query_all(doc, Document::ROOT, s).unwrap();
    assert_eq!(hits(&doc, ":focus"), [a]);
    assert_eq!(
        hits(&doc, "[id]:focus-within"),
        [doc.find_by_tag("div")[0], a]
    );
    doc.set_focus(Focus::Off);
    assert_eq!(focused_element(&doc), None);
    assert!(hits(&doc, ":focus").is_empty());
}

#[test]
fn focus_on_an_element_that_left_or_cannot_take_it_is_nowhere() {
    let mut doc = Document::parse("<input id=a><p id=p>p</p>");
    let (a, p) = (doc.find_by_tag("input")[0], doc.find_by_tag("p")[0]);
    let text = doc.node(p).children[0];
    assert!(!focusable(&doc, p, doc.element(p).unwrap()));
    doc.set_focus(Focus::On(p));
    assert_eq!(focused_element(&doc), None);
    doc.set_focus(Focus::On(text));
    assert_eq!(focused_element(&doc), None);
    doc.set_focus(Focus::On(a));
    doc.detach(a);
    assert_eq!(focused_element(&doc), None);
}
