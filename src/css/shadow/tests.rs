use super::*;
use crate::css::selector::Specificity;
use crate::css::selparse::parse_selector_list;

fn sel(s: &str) -> Selector {
    parse_selector_list(s).remove(0)
}

/// The element with `id` under `from`, shadow roots included.
fn find(doc: &Document, id: &str) -> NodeId {
    (0..doc.len() as NodeId)
        .find(|&n| doc.element(n).is_some_and(|e| e.attr("id") == Some(id)))
        .unwrap()
}

const PAGE: &str = "<div id=h class=dark><template shadowrootmode=open>\
    <p id=p><slot id=s name=a></slot></p><i id=i>x</i></template>\
    <b id=b slot=a class=t>slotted</b><u id=u>no slot</u></div><p id=out>o</p>";

#[test]
fn host_matches_only_the_context_host_and_only_as_host() {
    let doc = Document::parse(PAGE);
    let (h, p, out) = (find(&doc, "h"), find(&doc, "p"), find(&doc, "out"));
    let cx = Context {
        host: Some(h),
        ..Context::of(&doc)
    };
    let hit = |s: &str, at| matches_in(&sel(s), cx, at);
    assert!(hit(":host", h));
    assert!(hit(":host(.dark)", h));
    assert!(!hit(":host(.light)", h));
    // Featureless: nothing but :host matches the host from inside.
    assert!(!hit("div", h));
    assert!(!hit(".dark", h));
    assert!(!hit(":host", p));
    // Combinators climb out of the tree to the host, and no further.
    assert!(hit(":host p", p));
    assert!(hit(":host(.dark) > p", p));
    assert!(!hit("div p", p));
    assert!(!hit("body p", p));
    // Outside a shadow tree's sheet, :host is nothing.
    assert!(!matches_in(&sel(":host"), Context::of(&doc), h));
    assert!(!matches_in(&sel(":host"), cx, out));
}

#[test]
fn slotted_matches_the_slot_for_the_element_assigned_to_it() {
    let doc = Document::parse(PAGE);
    let (h, s, b, u) = (
        find(&doc, "h"),
        find(&doc, "s"),
        find(&doc, "b"),
        find(&doc, "u"),
    );
    let cx = |e| Context {
        host: Some(h),
        slotted: Some(e),
        ..Context::of(&doc)
    };
    assert!(matches_in(&sel("::slotted(.t)"), cx(b), s));
    assert!(matches_in(&sel("slot[name=a]::slotted(b)"), cx(b), s));
    assert!(matches_in(&sel(":host p > slot::slotted(*)"), cx(b), s));
    assert!(!matches_in(&sel("::slotted(i)"), cx(b), s));
    assert!(!matches_in(&sel("::slotted(*)"), cx(u), s));
    assert!(!matches_in(&sel("::slotted(*)"), Context::of(&doc), s));
}

#[test]
fn host_and_slotted_are_as_specific_as_their_kind_plus_their_argument() {
    assert_eq!(sel(":host").specificity(), Specificity(0, 1, 0));
    assert_eq!(sel(":host(#a.b)").specificity(), Specificity(1, 2, 0));
    assert_eq!(sel("::slotted(p.q)").specificity(), Specificity(0, 1, 2));
    assert!(sel(":host(p)").evaluable());
    assert!(sel("::slotted(p)").evaluable());
    // The argument is one compound: a combinator makes the whole unsupported.
    assert!(!sel(":host(div p)").evaluable());
    assert!(!sel("::slotted(a b)").evaluable());
    assert!(!sel(":slotted(p)").evaluable());
}

#[test]
fn host_and_slotted_parse_into_their_simples() {
    let host = sel(":host(.a)");
    assert_eq!(host.parts[0].1.simples, [Simple::Host(vec![sel(".a")])]);
    assert_eq!(sel(":host").parts[0].1.simples, [Simple::Host(Vec::new())]);
    let slotted = sel("::slotted(p)");
    assert_eq!(
        slotted.parts[0].1.simples,
        [Simple::Slotted(vec![sel("p")])]
    );
    assert_ne!(host, slotted);
}
