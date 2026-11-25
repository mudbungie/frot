use super::{AttrOp, AttrTest};
use crate::css::selector::Simple;
use crate::css::selparse::parse_selector_list;

/// The attribute test of the one simple in `sel`.
fn test(sel: &str) -> Option<AttrTest> {
    let list = parse_selector_list(sel);
    match &list[0].parts[0].1.simples[0] {
        Simple::Attr { test, .. } => test.clone(),
        other => panic!("{sel}: {other:?}"),
    }
}

fn holds(sel: &str, v: &str) -> bool {
    test(sel).expect("an operator").holds(v)
}

#[test]
fn every_operator_parses_with_its_flag() {
    for (sel, op, val, fold) in [
        ("[a=v]", AttrOp::Equals, "v", false),
        ("[a~=v]", AttrOp::Includes, "v", false),
        ("[a|=v]", AttrOp::DashMatch, "v", false),
        ("[a ^= 'v w' ]", AttrOp::Prefix, "v w", false),
        ("[a$=\"v\" i]", AttrOp::Suffix, "v", true),
        ("[a*=v S]", AttrOp::Substring, "v", false),
        ("[a='V'I]", AttrOp::Equals, "V", true),
    ] {
        assert_eq!(
            test(sel),
            Some(AttrTest {
                op,
                val: val.into(),
                fold
            }),
            "{sel}"
        );
    }
    assert_eq!(test("[a]"), None);
}

#[test]
fn each_operator_matches_as_selectors_4_says() {
    // (selector, value, matches) — the Selectors 4 §6 examples and edges.
    for (sel, v, want) in [
        ("[a~=b]", "a b c", true),
        ("[a~=b]", "abc", false),
        ("[a~='b c']", "b c", false),
        ("[a~='']", "", false),
        ("[a|=en]", "en", true),
        ("[a|=en]", "en-US", true),
        ("[a|=en]", "english", false),
        ("[a^=ht]", "http", true),
        ("[a^=tp]", "http", false),
        ("[a^='']", "x", false),
        ("[a$=js]", "x.js", true),
        ("[a$=js]", "x.json", false),
        ("[a$='']", "x", false),
        ("[a*=SDK]", "otSDKStub.js", true),
        ("[a*=sdk]", "otSDKStub.js", false),
        ("[a*=sdk i]", "otSDKStub.js", true),
        ("[a*='']", "x", false),
        ("[a=x i]", "X", true),
        ("[a=x]", "X", false),
    ] {
        assert_eq!(holds(sel, v), want, "{sel} on {v:?}");
    }
}
