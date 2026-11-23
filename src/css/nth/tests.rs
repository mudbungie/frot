use super::{Among, Nth};
use crate::css::query_all;
use crate::css::selector::{Selector, Simple, Specificity};
use crate::css::selparse::parse_selector_list;
use crate::dom::{Document, NodeKind};

fn sel(s: &str) -> Selector {
    parse_selector_list(s).pop().unwrap()
}

fn first(s: &str) -> Simple {
    sel(s).parts[0].1.simples[0].clone()
}

/// The `(a, b)` that `:nth-child(formula)` parses to, if it parses.
fn formula(formula: &str) -> Option<(i32, i32)> {
    match first(&format!(":nth-child({formula})")) {
        Simple::Nth(n) => Some((n.a, n.b)),
        _ => None,
    }
}

const LIST: &str = "<ul><li id=a></li><!-- c --><li id=b class=x></li>text<span id=s></span>\
                    <li id=c></li><li id=d class=x></li><li id=e class=x></li></ul>";

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

#[test]
fn an_plus_b_is_the_grammar_chrome_accepts() {
    for (text, want) in [
        ("odd", (2, 1)),
        ("EVEN", (2, 0)),
        ("3", (0, 3)),
        ("+3", (0, 3)),
        ("-1", (0, -1)),
        ("n", (1, 0)),
        ("+n", (1, 0)),
        ("-n", (-1, 0)),
        ("2N", (2, 0)),
        ("+2n", (2, 0)),
        (" 2n + 1 ", (2, 1)),
        ("2n+ 1", (2, 1)),
        ("2n +1", (2, 1)),
        ("n- 1", (1, -1)),
        ("n -1", (1, -1)),
        ("3n-2", (3, -2)),
        ("-2n+6", (-2, 6)),
        ("-0n+1", (0, 1)),
        ("-n-1", (-1, -1)),
    ] {
        assert_eq!(formula(text), Some(want), "{text}");
    }
    for bad in [
        "",
        "2n1",
        "2 n+1",
        "2 n",
        "+ n",
        "- n",
        "--n",
        "n+",
        "n+ +1",
        "n - -1",
        "2.5",
        "x",
        "n*2",
        "99999999999",
        "99999999999n",
        "n+99999999999",
    ] {
        assert_eq!(formula(bad), None, "{bad}");
    }
}

#[test]
fn the_named_forms_are_fixed_cases() {
    let named = |s: &str| match first(s) {
        Simple::Nth(n) => (n.a, n.b, n.start, n.end, n.among),
        other => panic!("{s}: {other:?}"),
    };
    assert_eq!(named(":first-child"), (0, 1, true, false, Among::Children));
    assert_eq!(named(":LAST-child"), (0, 1, false, true, Among::Children));
    assert_eq!(named(":only-child"), (0, 1, true, true, Among::Children));
    assert_eq!(named(":first-of-type"), (0, 1, true, false, Among::Type));
    assert_eq!(named(":last-of-type"), (0, 1, false, true, Among::Type));
    assert_eq!(named(":only-of-type"), (0, 1, true, true, Among::Type));
    assert_eq!(named(":nth-child(2)"), (0, 2, true, false, Among::Children));
    assert_eq!(
        named(":nth-last-child(2)"),
        (0, 2, false, true, Among::Children)
    );
    assert_eq!(named(":nth-of-type(2)"), (0, 2, true, false, Among::Type));
    assert_eq!(
        named(":nth-last-of-type(2)"),
        (0, 2, false, true, Among::Type)
    );
    assert_eq!(
        named(":nth-child(2 of .x, b)"),
        (0, 2, true, false, Among::Of(vec![sel(".x"), sel("b")]))
    );
    assert_eq!(Nth::named("root"), None);
    assert_eq!(Nth::function("is", "a"), None);
}

#[test]
fn what_is_not_this_grammar_is_unsupported() {
    for bad in [
        ":nth-child",
        ":first-child()",
        "::nth-child(1)",
        "::only-child",
        ":nth-child(2 OF li)",
        ":nth-child(2of li)",
        ":nth-child(2 of )",
        ":nth-child(2 of)",
        ":nth-child(of li)",
        ":nth-child(2 offset)",
        ":nth-child(2 of a::before)",
        ":nth-of-type(2 of li)",
        ":nth-last-of-type(2 of li)",
        ":nth-col(2)",
    ] {
        assert_eq!(first(bad), Simple::Unsupported, "{bad}");
    }
}

#[test]
fn positions_count_element_siblings_from_either_end() {
    let hit = |s: &str| ids(LIST, &format!("ul > {s}"));
    assert_eq!(hit(":nth-child(odd)"), "a s d");
    assert_eq!(hit(":nth-child(2n)"), "b c e");
    assert_eq!(hit(":nth-child(-n+2)"), "a b");
    assert_eq!(hit(":nth-child(n+5)"), "d e");
    assert_eq!(hit(":nth-child(-2n+6)"), "b c e");
    assert_eq!(hit(":nth-child(0)"), "");
    assert_eq!(hit(":nth-child(-n)"), "");
    assert_eq!(hit(":nth-child(n-1)"), "a b s c d e");
    assert_eq!(hit(":nth-last-child(1)"), "e");
    assert_eq!(hit(":nth-last-child(-n+2)"), "d e");
    assert_eq!(hit(":first-child"), "a");
    assert_eq!(hit(":last-child"), "e");
    assert_eq!(hit(":only-child"), "");
    assert_eq!(ids("<div><p id=a>x</p> </div>", "p:only-child"), "a");
    assert_eq!(hit(":not(:nth-child(odd))"), "b c e");
}

#[test]
fn of_type_counts_siblings_of_the_same_name() {
    let hit = |s: &str| ids(LIST, &format!("ul > {s}"));
    assert_eq!(hit(":nth-of-type(2)"), "b");
    assert_eq!(hit(":nth-of-type(odd)"), "a s c e");
    assert_eq!(hit(":nth-last-of-type(2)"), "d");
    assert_eq!(hit(":first-of-type"), "a s");
    assert_eq!(hit(":last-of-type"), "s e");
    assert_eq!(hit(":only-of-type"), "s");
}

#[test]
fn of_a_selector_list_counts_only_the_siblings_that_match_it() {
    let hit = |s: &str| ids(LIST, &format!("ul > {s}"));
    assert_eq!(hit(":nth-child(2 of .x)"), "d");
    assert_eq!(hit(":nth-child(odd of li)"), "a c e");
    assert_eq!(hit(":nth-child(2 of .x, span)"), "s");
    assert_eq!(hit(":nth-last-child(1 of li.x)"), "e");
    assert_eq!(hit(":nth-child(n of #s)"), "s");
    assert_eq!(hit(":nth-child(1 of [title=' of '])"), "");
}

#[test]
fn it_weighs_as_a_class_plus_its_of_list_and_fails_closed_on_an_unknown() {
    let spec = |s: &str| sel(s).specificity();
    assert_eq!(spec("li:nth-child(2)"), Specificity(0, 1, 1));
    assert_eq!(spec("li:only-of-type"), Specificity(0, 1, 1));
    assert_eq!(spec(":nth-child(2 of #a, .b)"), Specificity(1, 1, 0));
    assert!(sel(":not(:nth-child(2 of .x))").evaluable());
    assert!(!sel(":nth-child(2 of :unknown)").evaluable());
    assert_eq!(
        first(":not(:nth-child(2 of :unknown))"),
        Simple::Unsupported
    );
}
