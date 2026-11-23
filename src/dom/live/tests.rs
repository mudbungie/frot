use super::Focus;
use crate::dom::Document;

#[test]
fn a_parsed_document_has_no_script_state() {
    let doc = Document::parse("<div popover></div><dialog></dialog>");
    assert_eq!(doc.focus(), Focus::Auto);
    let div = doc.find_by_tag("div")[0];
    assert!(!doc.popover_open(div) && !doc.modal(div));
}

#[test]
fn state_is_set_and_cleared_per_element() {
    let mut doc = Document::parse("<div popover></div><dialog></dialog>");
    let (div, dialog) = (doc.find_by_tag("div")[0], doc.find_by_tag("dialog")[0]);
    doc.set_focus(Focus::On(div));
    assert_eq!(doc.focus(), Focus::On(div));
    assert_eq!(doc.generation(), 1);
    doc.set_popover(div, true);
    doc.set_popover(div, true);
    doc.set_modal(dialog, true);
    assert!(doc.popover_open(div) && doc.modal(dialog) && !doc.modal(div));
    doc.set_popover(div, false);
    doc.set_modal(dialog, false);
    assert!(!doc.popover_open(div) && !doc.modal(dialog));
}
