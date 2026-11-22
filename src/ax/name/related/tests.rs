//! `title` against the related element that is a node's whole native name
//! (`bl-dfc3`, Chrome 158).

use crate::ax::accessible_name;
use crate::dom::Document;

fn name(html: &str, tag: &str, css: bool) -> Option<String> {
    let doc = Document::parse(html);
    let styles = css.then(|| crate::css::compute(&doc));
    accessible_name(&doc, doc.find_by_tag(tag)[0], styles.as_ref())
}

const ROWS: &str = "<tr><th>a</th><th>b</th></tr>";

#[test]
fn a_caption_shuts_title_out_even_when_it_says_nothing() {
    for caption in [
        "<caption></caption>",
        "<caption> </caption>",
        "<caption><span style='display:none'>h</span></caption>",
    ] {
        let html = format!("<table title=T summary=S>{caption}{ROWS}</table>");
        assert_eq!(name(&html, "table", true), None, "{html}");
    }
    let html = format!("<table title=T>{ROWS}</table>");
    assert_eq!(name(&html, "table", true).as_deref(), Some("T"));
}

#[test]
fn a_hidden_caption_is_no_caption() {
    for hide in [
        "hidden",
        "aria-hidden=true",
        "style='display:none'",
        "style='visibility:hidden'",
    ] {
        let html = format!("<table title=T><caption {hide}>c</caption>{ROWS}</table>");
        assert_eq!(name(&html, "table", true).as_deref(), Some("T"), "{html}");
        let html = format!("<table summary=S><caption {hide}>c</caption>{ROWS}</table>");
        assert_eq!(name(&html, "table", true).as_deref(), Some("S"), "{html}");
        let html =
            format!("<table><caption {hide}>c</caption><caption>next</caption>{ROWS}</table>");
        assert_eq!(
            name(&html, "table", true).as_deref(),
            Some("next"),
            "{html}"
        );
    }
}

#[test]
fn a_legend_shuts_title_out_hidden_or_not() {
    for legend in ["<legend></legend>", "<legend hidden>x</legend>"] {
        let html = format!("<fieldset title=T>{legend}</fieldset>");
        assert_eq!(name(&html, "fieldset", true), None, "{html}");
    }
    let html = "<fieldset title=T></fieldset>";
    assert_eq!(name(html, "fieldset", false).as_deref(), Some("T"));
}

#[test]
fn title_does_not_name_a_caption_but_does_a_figcaption() {
    let html = format!("<table><caption title=CT></caption>{ROWS}</table>");
    assert_eq!(name(&html, "caption", true), None);
    assert_eq!(name("<div role=caption title=RT></div>", "div", true), None);
    let html = "<figure><figcaption title=FT></figcaption></figure>";
    assert_eq!(name(html, "figcaption", true).as_deref(), Some("FT"));
    let html = format!("<table><caption aria-label=CL></caption>{ROWS}</table>");
    assert_eq!(name(&html, "caption", true).as_deref(), Some("CL"));
}
