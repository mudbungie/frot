use crate::css::selector::{matches, Selector, Simple, Specificity};
use crate::css::selparse::parse_selector_list;
use crate::dom::Document;

fn sel(s: &str) -> Selector {
    parse_selector_list(s).pop().unwrap()
}

fn first(s: &str) -> Simple {
    sel(s).parts[0].1.simples[0].clone()
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
        "::is(a)",
        "::where(a)",
        "::not(a)",
        "::first-child",
        "::last-child",
        ":only-child",
        ":first-of-type",
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
    let doc = Document::parse("<div class=wrap><p class='x y'>t</p></div>");
    let p = doc.find_by_tag("p")[0];
    let hit = |s: &str| matches(&sel(s), &doc, p);
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

#[test]
fn first_and_last_child_parse_and_weigh_as_a_class() {
    assert_eq!(first(":first-child"), Simple::FirstChild);
    assert_eq!(first(":LAST-CHILD"), Simple::LastChild);
    assert_eq!(sel("li:first-child").specificity(), Specificity(0, 1, 1));
    assert_eq!(
        sel("li:last-child::after").specificity(),
        Specificity(0, 1, 2)
    );
    assert!(sel("li:first-child:not(:last-child)").evaluable());
}

#[test]
fn first_and_last_child_count_element_siblings_only() {
    let doc = Document::parse(
        "<ul> text <!-- c --><li id=a>1</li> <li id=b>2</li><li id=c>3</li> tail </ul>\
         <ol><li id=only>x</li></ol>",
    );
    let hit = |s: &str, id: &str| {
        let t = crate::css::query_all(&doc, Document::ROOT, &format!("#{id}")).unwrap()[0];
        matches(&sel(s), &doc, t)
    };
    assert!(hit("li:first-child", "a"));
    assert!(!hit("li:first-child", "b"));
    assert!(!hit("li:last-child", "b"));
    assert!(hit("li:last-child", "c"));
    assert!(!hit("li:first-child", "c"));
    assert!(hit("li:first-child:last-child", "only"));
    assert!(hit(":not(:first-child):not(:last-child)", "b"));
    assert!(hit("li:first-child + li", "b"));
    assert!(hit("ul > :last-child", "c"));
    // The root element is its parent's — the document's — only element child.
    let html = doc.find_by_tag("html")[0];
    assert!(matches(&sel(":first-child:last-child"), &doc, html));
}

#[test]
fn a_node_that_is_not_in_the_tree_has_no_siblings() {
    let mut doc = Document::parse("<p>x</p>");
    let loose = doc.create_element("i");
    assert!(matches(&sel("i:first-child:last-child"), &doc, loose));
    assert!(!matches(&sel("* + i"), &doc, loose));
    assert!(!matches(&sel("* ~ i"), &doc, loose));
}
