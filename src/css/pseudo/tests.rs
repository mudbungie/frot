use crate::css::selector::{matches, Selector, Simple, Specificity};
use crate::css::selparse::parse_selector_list;
use crate::dom::{Attr, Element};

fn sel(s: &str) -> Selector {
    parse_selector_list(s).pop().unwrap()
}

fn first(s: &str) -> Simple {
    sel(s).parts[0].1.simples[0].clone()
}

fn el(name: &str, class: &str) -> Element {
    Element {
        name: name.into(),
        attrs: vec![Attr {
            name: "class".into(),
            value: class.into(),
        }],
    }
}

#[test]
fn logical_pseudo_classes_parse_their_selector_lists() {
    assert_eq!(first(":is(a, .b)"), Simple::Is(vec![sel("a"), sel(".b")]));
    assert_eq!(first(":WHERE( a )"), Simple::Where(vec![sel("a")]));
    assert_eq!(
        first(":not(a > b, #c)"),
        Simple::Not(vec![sel("a > b"), sel("#c")])
    );
    // Nested, and with a comma inside an argument's own brackets.
    assert_eq!(
        first(":not(:is([a='x,y'], b))"),
        Simple::Not(vec![sel(":is([a='x,y'], b)")])
    );
    // A double colon is tolerated as it always was for pseudo-classes.
    assert_eq!(first("::is(a)"), Simple::Is(vec![sel("a")]));
}

#[test]
fn a_logical_pseudo_class_the_engine_cannot_answer_is_unsupported() {
    for bad in [
        ":is()",
        ":is(a,)",
        ":is(.)",
        ":is(a::before)",
        ":not(a::after)",
        ":where(a",
        ":not(:hover)",
        ":not(a, b:hover)",
        ":not(:is(a, :hover))",
        ":not(:where(:nth-child(2)))",
        ":not(:not(:hover))",
        ":has(a)",
        ":nth-child(2)",
    ] {
        assert_eq!(first(bad), Simple::Unsupported, "{bad}");
    }
    // Inside :is()/:where() an unsupported argument just never matches.
    assert_eq!(
        first(":is(a, :hover)"),
        Simple::Is(vec![sel("a"), sel(":hover")])
    );
}

#[test]
fn matching_follows_the_arguments() {
    let p = el("p", "x y");
    let div = el("div", "wrap");
    let hit = |s: &str| matches(&sel(s), &p, &[&div]);
    assert!(hit(":is(p, a)"));
    assert!(hit("p:is(.x)"));
    assert!(!hit(":is(a, .z)"));
    assert!(hit(":where(.y)"));
    assert!(!hit(":where(a)"));
    assert!(hit(":not(a)"));
    assert!(hit(":not(a, .z)"));
    assert!(!hit(":not(a, .x)"));
    assert!(hit("p:not([hidden])"));
    assert!(!hit(".x:not(p)"));
    // An argument is a complex selector, matched with the element's own context.
    assert!(hit(":is(.wrap > p)"));
    assert!(!hit(":not(div p)"));
    assert!(hit(":not(section p)"));
    // …and a logical pseudo-class works in any compound of the selector.
    assert!(hit(":not(.nope) > :is(p)"));
    assert!(!hit(":not(.wrap) > p"));
    // An unsupported argument never matches; a supported sibling still can.
    assert!(hit(":is(:hover, p)"));
    assert!(!hit(":is(:hover)"));
    assert!(hit(":not(:not(p))"));
}

#[test]
fn specificity_is_the_most_specific_argument_and_zero_for_where() {
    let spec = |s: &str| sel(s).specificity();
    assert_eq!(spec(":is(a, .b, #c)"), Specificity(1, 0, 0));
    assert_eq!(spec("p:not(.a.b, i)"), Specificity(0, 2, 1));
    assert_eq!(spec("p:where(#a, .b)"), Specificity(0, 0, 1));
    assert_eq!(spec(":is(:where(#a), b)"), Specificity(0, 0, 1));
    assert_eq!(spec(".x:not(:is(#a .b))::before"), Specificity(1, 2, 1));
}

#[test]
fn evaluable_sees_unsupported_at_any_depth() {
    assert!(sel("a:is(b, :where(.c)):not(d)").evaluable());
    assert!(!sel("a:hover").evaluable());
    assert!(!sel("a :is(b, :hover)").evaluable());
    assert!(!sel(":where(:is(:hover))").evaluable());
    assert!(sel("a::before").pseudo_element());
    assert!(sel("a::before b").pseudo_element());
    assert!(!sel("a b").pseudo_element());
}
