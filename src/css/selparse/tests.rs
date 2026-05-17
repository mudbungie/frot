use super::*;
use crate::css::selector::{Combinator, Pseudo, Simple};

fn one(s: &str) -> Selector {
    let mut v = parse_selector_list(s);
    assert_eq!(v.len(), 1, "expected exactly one selector from {s:?}");
    v.pop().unwrap()
}

fn simples(s: &str) -> Vec<Simple> {
    let sel = one(s);
    assert_eq!(sel.parts.len(), 1);
    sel.parts[0].1.simples.clone()
}

#[test]
fn split_top_respects_quotes_and_parens() {
    // comma inside quotes / parens does not split the list.
    let v = parse_selector_list(r#"[a="x,y"] , p:not(a,b) , div"#);
    assert_eq!(v.len(), 3);
    // trailing/empty entries are dropped.
    assert_eq!(parse_selector_list("a , , div").len(), 2);
    // a stray top-level ']' just stays in the chunk (then fails to parse).
    assert!(parse_selector_list("a]").is_empty());
    assert!(parse_selector_list("").is_empty());
}

#[test]
fn simple_selector_kinds() {
    assert_eq!(simples("*"), vec![Simple::Universal]);
    assert_eq!(simples("DIV"), vec![Simple::Type("div".into())]);
    assert_eq!(simples(".a_b"), vec![Simple::Class("a_b".into())]);
    assert_eq!(simples("#x-1"), vec![Simple::Id("x-1".into())]);
    assert_eq!(
        simples("div.a#i"),
        vec![
            Simple::Type("div".into()),
            Simple::Class("a".into()),
            Simple::Id("i".into()),
        ]
    );
}

#[test]
fn malformed_compounds_are_dropped() {
    for bad in [".", "#", ".{", ">"] {
        assert!(parse_selector_list(bad).is_empty(), "{bad:?} should drop");
    }
}

#[test]
fn lenient_about_bare_pseudo_and_trailing_combinator() {
    // ":" is kept as an Unsupported simple (never matches, not dropped).
    let s = one(":");
    assert!(s.parts[0].1.simples.contains(&Simple::Unsupported));
    // A dangling combinator keeps the leading compound.
    assert_eq!(one("a >").parts.len(), 1);
}

#[test]
fn attribute_selectors() {
    assert_eq!(
        simples("[data-k]"),
        vec![Simple::Attr {
            name: "data-k".into(),
            val: None
        }]
    );
    assert_eq!(
        simples("[Data-X = y ]"),
        vec![Simple::Attr {
            name: "data-x".into(),
            val: Some("y".into())
        }]
    );
    assert_eq!(
        simples(r#"[a="b c"]"#),
        vec![Simple::Attr {
            name: "a".into(),
            val: Some("b c".into())
        }]
    );
    assert_eq!(
        simples("[a=]"),
        vec![Simple::Attr {
            name: "a".into(),
            val: Some(String::new())
        }]
    );
}

#[test]
fn malformed_attribute_selectors_drop() {
    for bad in ["[=x]", "[a~=b]", "[a", "[a=b", r#"[a="b]"#, "[a b]"] {
        assert!(parse_selector_list(bad).is_empty(), "{bad:?} should drop");
    }
}

#[test]
fn pseudo_elements_and_unsupported_pseudos() {
    assert_eq!(one("p:before").pseudo(), Some(Pseudo::Before));
    assert_eq!(one("p::before").pseudo(), Some(Pseudo::Before));
    assert_eq!(one("p::after").pseudo(), Some(Pseudo::After));
    assert_eq!(one("::before").pseudo(), Some(Pseudo::Before));
    // unsupported pseudo-classes become an Unsupported simple, no pseudo.
    let s = one("p:hover");
    assert_eq!(s.pseudo(), None);
    assert!(s.parts[0].1.simples.contains(&Simple::Unsupported));
    assert!(one("p::first-line").parts[0]
        .1
        .simples
        .contains(&Simple::Unsupported));
}

#[test]
fn functional_pseudos_consume_balanced_parens() {
    for fp in ["a:nth-child(2n+1)", "a:not(b(c))", "a:nth-child(2"] {
        let s = one(fp);
        assert!(s.parts[0].1.simples.contains(&Simple::Unsupported));
    }
}

#[test]
fn combinators() {
    let s = one("div > .a section span");
    assert_eq!(s.parts.len(), 4);
    assert_eq!(s.parts[0].0, Combinator::Descendant);
    assert_eq!(s.parts[1].0, Combinator::Child);
    assert_eq!(s.parts[2].0, Combinator::Descendant);
    assert_eq!(s.parts[3].0, Combinator::Descendant);
    // leading whitespace flushes an empty buffer (no spurious compound).
    assert_eq!(one("   p").parts.len(), 1);
}

#[test]
fn split_top_is_quote_aware_directly() {
    assert_eq!(split_top("a;b", ';'), vec!["a", "b"]);
    assert_eq!(split_top(r#""a;b""#, ';'), vec![r#""a;b""#]);
    assert_eq!(split_top("x)", ';'), vec!["x)"]);
}
