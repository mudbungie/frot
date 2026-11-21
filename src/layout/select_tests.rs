//! Every row is a Chrome 158 measurement (`getClientRects().length` of each
//! descendant, `bl-2932`).

use crate::dom::Document;
use crate::layout::{compute, VIEWPORT_WIDTH};

/// Tags of the elements under `<body>` that get a box, in document order.
fn boxed(html: &str) -> Vec<String> {
    let doc = Document::parse(html);
    let styles = crate::css::compute(&doc);
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    ["select", "optgroup", "option", "hr", "span"]
        .into_iter()
        .flat_map(|tag| doc.find_by_tag(tag).into_iter().map(move |id| (tag, id)))
        .filter(|&(_, id)| layout.rect(id).is_some())
        .map(|(tag, _)| tag.to_string())
        .collect()
}

#[test]
fn a_menu_list_boxes_only_itself() {
    for select in [
        "<select>",
        "<select size=1>",
        "<select size=0>",
        "<select size=x>",
        "<select size=-2>",
        "<select size=1.5>",
        "<select size=99999999999999999999>",
        "<select multiple size=1>",
        "<select multiple size=01>",
        "<select style='display:block'>",
    ] {
        let html = format!("{select}<optgroup label=G><option>a</optgroup><option>b<hr></select>");
        assert_eq!(boxed(&html), ["select"], "{select}");
    }
}

#[test]
fn a_list_box_lays_its_contents_out() {
    for select in [
        "<select size=2>",
        "<select size=' 4 '>",
        "<select size=+3>",
        "<select size=2abc>",
        "<select multiple>",
        "<select multiple=false>",
        "<select multiple size=0>",
        "<select multiple size=-5>",
    ] {
        let html = format!("{select}<optgroup label=G><option>a</optgroup><option>b</select>");
        assert_eq!(
            boxed(&html),
            ["select", "optgroup", "option", "option"],
            "{select}"
        );
    }
}

#[test]
fn an_option_outside_a_select_is_an_ordinary_block() {
    assert_eq!(boxed("<div><option>a</option></div>"), ["option"]);
}

#[test]
fn a_menu_lists_text_sizes_nothing() {
    // The select keeps a box, but its options' words are not measured: the
    // paragraph is one line high and the select has no width.
    let doc = Document::parse("<p>x <select><option>long option text</select></p>");
    let styles = crate::css::compute(&doc);
    let layout = compute(&doc, &styles, VIEWPORT_WIDTH);
    let select = layout.rect(doc.find_by_tag("select")[0]).unwrap();
    assert_eq!((select.w, select.h), (0, 0));
}
