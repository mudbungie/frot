//! The index is an optimization, so its tests are equivalence tests: the two
//! invariants in the module doc (completeness, order) are checked mechanically
//! against the exhaustive flat scan the index replaces.

use super::Index;
use crate::css::parse::Stylesheet;
use crate::css::selector::matches;
use crate::dom::{Document, Element, NodeId, NodeKind};

/// Selector shapes that between them hit every bucket: id, class, tag, and the
/// keyless `others` (universal, attribute-only, unsupported pseudo).
const SHEET: &str = "
    #hero{display:none} #hero.k{display:block}
    .k{display:none} div.k span{display:block} .k>b{display:none}
    p{display:none} p span{display:block}
    *{visibility:visible} [data-x]{display:none} [data-x=1]{display:block}
    :hover{display:none} a:nth-child(2){display:none}
    .k,#hero,em{content:'x'} b::before{content:'y'}
";

const HTML: &str = "<div id=hero class='k extra'><p><span>a</span><b data-x=1>b</b></p>
    <em class=k>e</em><a href=#>l</a><i>i</i></div>";

fn each_element(doc: &Document, mut f: impl FnMut(&Element, &[&Element])) {
    fn walk<'a>(
        doc: &'a Document,
        id: NodeId,
        anc: &mut Vec<&'a Element>,
        f: &mut impl FnMut(&Element, &[&Element]),
    ) {
        let NodeKind::Element(el) = &doc.node(id).kind else {
            return;
        };
        let near: Vec<&Element> = anc.iter().rev().copied().collect();
        f(el, &near);
        anc.push(el);
        for &c in &doc.node(id).children {
            walk(doc, c, anc, f);
        }
        anc.pop();
    }
    let mut anc = Vec::new();
    for &r in doc.roots() {
        walk(doc, r, &mut anc, &mut f);
    }
}

/// Completeness: every (rule, selector) pair the exhaustive scan would find
/// matching is offered as a candidate. If this holds for every element, the
/// indexed cascade sees exactly the declarations the flat scan saw.
#[test]
fn candidates_cover_every_match() {
    let sheets = vec![Stylesheet::parse(SHEET)];
    let index = Index::build(&sheets);
    let doc = Document::parse(HTML);
    let mut matched = 0usize;
    each_element(&doc, |el, anc| {
        let offered: Vec<usize> = index.candidates(el).iter().map(|c| c.seq).collect();
        let mut seq = 0usize;
        for sheet in &sheets {
            for rule in &sheet.rules {
                for sel in &rule.selectors {
                    if matches(sel, el, anc) {
                        assert!(offered.contains(&seq), "{} missed seq {seq}", el.name);
                        matched += 1;
                    }
                    seq += 1;
                }
            }
        }
    });
    // Guard against the assertion loop passing vacuously.
    assert!(matched > 10, "fixture matched too little: {matched}");
}

/// Order: candidates arrive in flat-scan sequence, which is what the cascade's
/// last-max tie-break depends on.
#[test]
fn candidates_are_in_flat_scan_order() {
    let sheets = vec![Stylesheet::parse(SHEET), Stylesheet::parse(".k{display:none}")];
    let index = Index::build(&sheets);
    let doc = Document::parse(HTML);
    each_element(&doc, |el, _| {
        let seqs: Vec<usize> = index.candidates(el).iter().map(|c| c.seq).collect();
        assert!(seqs.windows(2).all(|w| w[0] < w[1]), "out of order: {seqs:?}");
    });
}

/// Keyless selectors stay in the always-tested bucket rather than vanishing:
/// `selector.rs` fails closed on unsupported grammar, and that only holds if
/// the rule is still evaluated.
#[test]
fn keyless_selectors_land_in_others() {
    let sheets = vec![Stylesheet::parse("*{display:none} [data-x]{display:none} :hover{color:x}")];
    let index = Index::build(&sheets);
    assert_eq!(index.others.len(), 3);
    assert!(index.ids.is_empty() && index.classes.is_empty() && index.tags.is_empty());
}

/// The subject compound picks the narrowest key it has: id over class over
/// tag. Ancestor compounds never key the rule.
#[test]
fn key_prefers_id_then_class_then_tag() {
    let sheets = vec![Stylesheet::parse("a div#i.c{a:b} a div.c{a:b} a div{a:b}")];
    let index = Index::build(&sheets);
    assert_eq!(index.ids["i"].len(), 1);
    assert_eq!(index.classes["c"].len(), 1);
    assert_eq!(index.tags["div"].len(), 1);
    assert!(!index.tags.contains_key("a"));
}

/// Source order across sheets is continuous, and inline `style=` sits one slot
/// past the last author rule.
#[test]
fn rule_count_spans_all_sheets() {
    let sheets = vec![Stylesheet::parse("a{x:y} b{x:y}"), Stylesheet::parse("c{x:y}")];
    let index = Index::build(&sheets);
    assert_eq!(index.rule_count, 3);
    assert_eq!(index.tags["c"][0].order, 2);
}
