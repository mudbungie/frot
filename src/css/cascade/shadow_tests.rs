//! Style scoping across shadow boundaries (js.md §15, css.md "Shadow trees").

use crate::css::{compute, compute_bare, Display, Styles};
use crate::dom::{Document, NodeId};

const PAGE: &str = "<style>p { display: inline-block } x-h { display: inline } \
    b { display: inline } slot { display: block }</style>\
    <x-h id=h><template shadowrootmode=open><style>:host { display: block } \
    p { display: none } ::slotted(i) { display: block } ::slotted(b) { display: flex !important } \
    slot { text-transform: uppercase }</style><p id=sp>s</p><slot id=s></slot></template>\
    <i id=i>i</i><b id=b>b</b><u id=u slot=none>u</u></x-h><p id=lp>l</p>";

fn find(doc: &Document, id: &str) -> NodeId {
    (0..doc.len() as NodeId)
        .find(|&n| doc.element(n).is_some_and(|e| e.attr("id") == Some(id)))
        .unwrap()
}

fn display(doc: &Document, styles: &Styles, id: &str) -> Display {
    styles.display(find(doc, id))
}

#[test]
fn each_tree_is_styled_by_its_own_sheets() {
    let doc = Document::parse(PAGE);
    let st = compute(&doc);
    // The document's `p` rule does not reach into the shadow tree, nor the
    // shadow tree's out of it.
    assert_eq!(display(&doc, &st, "sp"), Display::None);
    assert_eq!(display(&doc, &st, "lp"), Display::InlineBlock);
    // The shadow tree's `slot` rule wins in its tree; the document's slot
    // rule does not apply there.
    assert_eq!(display(&doc, &st, "s"), Display::Contents);
}

#[test]
fn host_and_slotted_rules_lose_normal_ties_to_the_outer_tree() {
    let doc = Document::parse(PAGE);
    let st = compute(&doc);
    // `x-h { display: inline }` (outer, normal) beats `:host { block }`.
    assert_eq!(display(&doc, &st, "h"), Display::Inline);
    // No outer rule: `::slotted(i)` applies.
    assert_eq!(display(&doc, &st, "i"), Display::Block);
    // An inner `!important` beats the outer normal `b` rule.
    assert_eq!(display(&doc, &st, "b"), Display::Flex);
    // Inherited through the flat tree: the slot, not the host.
    let i = find(&doc, "i");
    assert_eq!(st.text(i, "i").to_string(), "i");
    let text = doc.node(i).children[0];
    assert_eq!(st.text(text, "i"), "I");
}

#[test]
fn adopted_sheets_join_their_roots_sheets() {
    let mut doc = Document::parse(PAGE);
    let root = doc.shadow_root(find(&doc, "h")).unwrap();
    doc.set_adopted(Document::ROOT, vec!["#lp { display: none }".into()]);
    doc.set_adopted(root, vec!["p { display: block }".into()]);
    let st = compute(&doc);
    assert_eq!(display(&doc, &st, "lp"), Display::None);
    assert_eq!(display(&doc, &st, "sp"), Display::Block);
    assert_eq!(doc.adopted(find(&doc, "lp")), [] as [String; 0]);
}

#[test]
fn bare_styles_take_no_author_sheet_from_any_tree() {
    let doc = Document::parse(PAGE);
    let st = compute_bare(&doc, false);
    assert_eq!(display(&doc, &st, "sp"), Display::Block);
    assert_eq!(display(&doc, &st, "h"), Display::Inline);
}
