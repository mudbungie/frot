use super::*;
use crate::css::query_all;
use crate::css::selparse::parse_selector_list;
use crate::dom::Document;

/// Whether `selector` matches the element with `id="t"` in `html`.
fn hit(selector: &str, html: &str) -> bool {
    let doc = Document::parse(html);
    let t = query_all(&doc, Document::ROOT, "#t").unwrap()[0];
    matches(&sel(selector), &doc, t)
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
    assert_eq!(sel(":unknown").specificity(), Specificity(0, 0, 0));
    assert_eq!(sel(":target").specificity(), Specificity(0, 1, 0));
    assert!(sel("#i").specificity() > sel(".c.d").specificity());
}

#[test]
fn type_class_id_attr_matching() {
    let p = "<p id=t class='a b' data-k=v>x</p>";
    for yes in ["p", ".a", ".b.a", "#t", "[data-k]", "[data-k=v]", "*"] {
        assert!(hit(yes, p), "{yes}");
    }
    for no in ["div", ".z", "#y", "[data-k=w]", "[missing]"] {
        assert!(!hit(no, p), "{no}");
    }
}

#[test]
fn classless_element_has_empty_class_list() {
    assert!(!hit(".a", "<p id=t>x</p>"));
}

#[test]
fn unsupported_pseudo_never_matches() {
    assert!(!hit("p:unknown", "<p id=t>x</p>"));
}

#[test]
fn only_an_element_matches() {
    let doc = Document::parse("<p>text</p>");
    let p = doc.find_by_tag("p")[0];
    let text = doc.node(p).children[0];
    assert!(matches(&sel("*"), &doc, p));
    assert!(!matches(&sel("*"), &doc, text));
}

#[test]
fn descendant_and_child_combinators() {
    let html = "<div class=wrap><section><span id=t>x</span></section></div>";
    for yes in [
        "div span",
        ".wrap span",
        "section > span",
        "div > section > span",
    ] {
        assert!(hit(yes, html), "{yes}");
    }
    for no in ["div > span", "article span", "span span", "span > span"] {
        assert!(!hit(no, html), "{no}");
    }
    // The top of the tree has no parent to be a child of.
    let doc = Document::parse("<html></html>");
    let root = doc.find_by_tag("html")[0];
    assert!(!matches(&sel("a > html"), &doc, root));
    assert!(!matches(&sel("a html"), &doc, root));
    // …and its siblings are the other roots.
    assert!(!matches(&sel("a + html"), &doc, root));
    assert!(!matches(&sel("a ~ html"), &doc, root));
}

#[test]
fn descendant_backtracks_across_ancestors() {
    // `a i` must skip the nearer `b` and find `a`; `a > b i` must not stop at
    // the first `b` that fails its own left-hand side.
    assert!(hit("a i", "<a><b><i id=t>x</i></b></a>"));
    assert!(hit("u > b i", "<u><b><s><b><i id=t>x</i></b></s></b></u>"));
}

#[test]
fn sibling_combinators() {
    let html = "<div><h2>a</h2> text <!-- c --><p class=x>b</p><i>c</i><p id=t>d</p><b>e</b></div>";
    // `+`: the element immediately before, whatever text or comments intervene.
    assert!(hit("i + p", html));
    assert!(!hit("h2 + p", html));
    assert!(!hit(".x + p", html));
    assert!(!hit("b + p", html));
    // `~`: any earlier element sibling, never a later one.
    for yes in [
        "h2 ~ p",
        ".x ~ p",
        "i ~ p",
        "h2 ~ i ~ p",
        "h2 + .x ~ p",
        "div > h2 ~ p",
    ] {
        assert!(hit(yes, html), "{yes}");
    }
    for no in ["b ~ p", "div ~ p", "p ~ h2 ~ p", "i ~ .x ~ p"] {
        assert!(!hit(no, html), "{no}");
    }
    // A first child has no earlier sibling; siblings exist at the top level too.
    assert!(!hit("* + h2", "<div><h2 id=t>a</h2></div>"));
    assert!(!hit("* ~ h2", "<div><h2 id=t>a</h2></div>"));
    // Mixed with ancestry on either side of the sibling step.
    assert!(hit(
        "div h2 + p",
        "<div><section><h2>a</h2><p id=t>b</p></section></div>"
    ));
    assert!(hit("h2 + p > i", "<h2>a</h2><p><i id=t>b</i></p>"));
    assert!(!hit("h2 + p > i", "<h2>a</h2><u></u><p><i id=t>b</i></p>"));
}
