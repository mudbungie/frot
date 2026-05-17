use super::*;
use crate::css::selparse::parse_selector_list;
use crate::dom::{Attr, Element};

fn el(name: &str, attrs: &[(&str, &str)]) -> Element {
    Element {
        name: name.into(),
        attrs: attrs
            .iter()
            .map(|(n, v)| Attr {
                name: (*n).into(),
                value: (*v).into(),
            })
            .collect(),
    }
}

fn sel(s: &str) -> Selector {
    parse_selector_list(s).pop().unwrap()
}

#[test]
fn pseudo_of_subject_only() {
    assert_eq!(sel("a").pseudo(), None);
    assert_eq!(sel("a::before").pseudo(), Some(Pseudo::Before));
    assert_eq!(sel("div ::after").pseudo(), Some(Pseudo::After));
}

#[test]
fn specificity_counts_each_class() {
    assert_eq!(sel("*").specificity(), Specificity(0, 0, 0));
    assert_eq!(sel("div").specificity(), Specificity(0, 0, 1));
    assert_eq!(sel(".c").specificity(), Specificity(0, 1, 0));
    assert_eq!(sel("[x=y]").specificity(), Specificity(0, 1, 0));
    assert_eq!(sel("#i").specificity(), Specificity(1, 0, 0));
    assert_eq!(sel("p::before").specificity(), Specificity(0, 0, 2));
    assert_eq!(sel(":hover").specificity(), Specificity(0, 0, 0));
    assert!(sel("#i").specificity() > sel(".c.d").specificity());
}

#[test]
fn type_class_id_attr_matching() {
    let p = el("p", &[("class", "a b"), ("id", "x"), ("data-k", "v")]);
    assert!(matches(&sel("p"), &p, &[]));
    assert!(!matches(&sel("div"), &p, &[]));
    assert!(matches(&sel(".a"), &p, &[]));
    assert!(matches(&sel(".b.a"), &p, &[]));
    assert!(!matches(&sel(".z"), &p, &[]));
    assert!(matches(&sel("#x"), &p, &[]));
    assert!(!matches(&sel("#y"), &p, &[]));
    assert!(matches(&sel("[data-k]"), &p, &[]));
    assert!(matches(&sel("[data-k=v]"), &p, &[]));
    assert!(!matches(&sel("[data-k=w]"), &p, &[]));
    assert!(!matches(&sel("[missing]"), &p, &[]));
    assert!(matches(&sel("*"), &p, &[]));
}

#[test]
fn classless_element_has_empty_class_list() {
    assert!(!matches(&sel(".a"), &el("p", &[]), &[]));
}

#[test]
fn unsupported_pseudo_never_matches() {
    assert!(!matches(&sel("p:hover"), &el("p", &[]), &[]));
}

#[test]
fn descendant_and_child_combinators() {
    let div = el("div", &[("class", "wrap")]);
    let section = el("section", &[]);
    let span = el("span", &[]);
    // ancestors nearest-first: span <- section <- div
    let anc = [&section, &div];
    assert!(matches(&sel("div span"), &span, &anc));
    assert!(matches(&sel(".wrap span"), &span, &anc));
    assert!(matches(&sel("section > span"), &span, &anc));
    assert!(!matches(&sel("div > span"), &span, &anc));
    assert!(matches(&sel("div > section"), &section, &[&div]));
    assert!(!matches(&sel("article span"), &span, &anc));
    // child combinator with no ancestors at all
    assert!(!matches(&sel("a > span"), &span, &[]));
}

#[test]
fn descendant_backtracks_across_ancestors() {
    let a = el("a", &[]);
    let b = el("b", &[]);
    let target = el("i", &[]);
    // i with ancestors b <- a ; selector "a i" must skip b and find a.
    assert!(matches(&sel("a i"), &target, &[&b, &a]));
}
