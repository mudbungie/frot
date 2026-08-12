//! Role resolution: the harness, explicit `role=`, and the unconditional
//! (flat) tag mappings. Conditional mappings live in [`conditions`].

use super::*;
use crate::dom::WalkEvent;

/// The node marked `id="t"`, in a real parsed document — role resolution is
/// contextual, so a test states the context and points at one node in it.
fn marked(html: &str) -> (Document, NodeId) {
    let doc = Document::parse(html);
    let mut found = None;
    doc.walk(None, &mut |ev, e| {
        if let (WalkEvent::Enter(id), NodeKind::Element(el)) = (ev, &e.kind) {
            if found.is_none() && el.attr("id") == Some("t") {
                found = Some(id);
            }
        }
    });
    (doc, found.expect("fixture has no id=t element"))
}

/// Effective role of the `id="t"` element.
fn r(html: &str) -> Option<&'static str> {
    let (doc, id) = marked(html);
    role(&doc, id)
}

/// The `id="t"` element itself, for the element-only [`level`].
fn e(html: &str) -> Element {
    let (doc, id) = marked(html);
    match &doc.node(id).kind {
        NodeKind::Element(el) => el.clone(),
        _ => unreachable!(),
    }
}

#[test]
fn a_text_node_has_no_role() {
    let doc = Document::parse("<p id=t>text</p>");
    let (_, id) = marked("<p id=t>text</p>");
    let text = doc.node(id).children[0];
    assert_eq!(role(&doc, text), None);
}

#[test]
fn anchor_with_href_is_link() {
    assert_eq!(r("<a id=t href='/x'>L</a>"), Some("link"));
}

#[test]
fn anchor_without_href_is_generic() {
    assert_eq!(r("<a id=t>L</a>"), Some("generic"));
}

#[test]
fn area_with_href_is_link() {
    assert_eq!(r("<map><area id=t href='/x'></map>"), Some("link"));
}

#[test]
fn area_without_href_is_generic() {
    assert_eq!(r("<map><area id=t></map>"), Some("generic"));
}

#[test]
fn explicit_role_wins_when_recognized() {
    assert_eq!(r("<div id=t role=button>x</div>"), Some("button"));
}

#[test]
fn explicit_role_picks_first_token() {
    assert_eq!(
        r("<div id=t role='navigation menu'>x</div>"),
        Some("navigation")
    );
}

#[test]
fn explicit_unknown_role_falls_back_to_implicit() {
    assert_eq!(r("<div id=t role=wibble>x</div>"), Some("generic"));
}

#[test]
fn empty_explicit_role_falls_back_to_implicit() {
    assert_eq!(r("<div id=t role=''>x</div>"), Some("generic"));
}

#[test]
fn explicit_role_on_a_tag_with_no_implicit_role_still_applies() {
    assert_eq!(r("<br id=t role=separator>"), Some("separator"));
}

#[test]
fn buttons_are_buttons() {
    assert_eq!(r("<button id=t>x</button>"), Some("button"));
    assert_eq!(
        r("<details><summary id=t>x</summary></details>"),
        Some("button")
    );
}

#[test]
fn select_default_is_combobox() {
    assert_eq!(r("<select id=t></select>"), Some("combobox"));
}

#[test]
fn select_with_multiple_is_listbox() {
    assert_eq!(r("<select id=t multiple></select>"), Some("listbox"));
}

#[test]
fn select_with_size_gt_1_is_listbox() {
    assert_eq!(r("<select id=t size=5></select>"), Some("listbox"));
}

#[test]
fn select_with_size_1_is_combobox() {
    assert_eq!(r("<select id=t size=1></select>"), Some("combobox"));
}

#[test]
fn select_with_unparseable_size_is_combobox() {
    assert_eq!(r("<select id=t size=xx></select>"), Some("combobox"));
}

#[test]
fn headings_are_heading_with_levels() {
    for (tag, lvl) in [
        ("h1", 1u32),
        ("h2", 2),
        ("h3", 3),
        ("h4", 4),
        ("h5", 5),
        ("h6", 6),
    ] {
        let html = format!("<{tag} id=t>x</{tag}>");
        assert_eq!(r(&html), Some("heading"));
        assert_eq!(level(&e(&html)), Some(lvl));
    }
}

#[test]
fn lists_are_lists() {
    for tag in ["ul", "ol", "dl", "menu"] {
        let html = format!("<{tag} id=t></{tag}>");
        assert_eq!(r(&html), Some("list"), "{tag}");
    }
}

#[test]
fn table_subroles() {
    assert_eq!(r("<table id=t><tr><td>d</td></tr></table>"), Some("table"));
    assert_eq!(r("<table><tr id=t><td>d</td></tr></table>"), Some("row"));
    assert_eq!(r("<table><tr><td id=t>d</td></tr></table>"), Some("cell"));
    for tag in ["thead", "tbody", "tfoot"] {
        let html = format!("<table><{tag} id=t><tr><td>d</td></tr></{tag}></table>");
        assert_eq!(r(&html), Some("rowgroup"), "{tag}");
    }
}

#[test]
fn structural_tags_have_no_role() {
    for html in [
        "<br id=t>",
        "<meta id=t>",
        "<link id=t>",
        "<title id=t>x</title>",
        "<script id=t></script>",
        "<style id=t></style>",
        "<head id=t></head>",
        "<html id=t></html>",
        "<wbr id=t>",
    ] {
        assert_eq!(r(html), None, "{html}");
    }
}

#[test]
fn inline_text_roles() {
    for (html, want) in [
        ("<strong id=t>x</strong>", "strong"),
        ("<em id=t>x</em>", "emphasis"),
        ("<code id=t>x</code>", "code"),
        ("<mark id=t>x</mark>", "mark"),
        ("<sub id=t>x</sub>", "subscript"),
        ("<sup id=t>x</sup>", "superscript"),
        ("<p id=t>x</p>", "paragraph"),
        ("<ins id=t>x</ins>", "insertion"),
        ("<del id=t>x</del>", "deletion"),
        ("<s id=t>x</s>", "deletion"),
        ("<dfn id=t>x</dfn>", "term"),
        ("<dl><dt id=t>x</dt></dl>", "term"),
        ("<dl><dd id=t>x</dd></dl>", "definition"),
        ("<dialog id=t>x</dialog>", "dialog"),
        ("<blockquote id=t>x</blockquote>", "blockquote"),
    ] {
        assert_eq!(r(html), Some(want), "{html}");
    }
}

#[test]
fn misc_roles() {
    for (html, want) in [
        ("<hr id=t>", "separator"),
        ("<textarea id=t></textarea>", "textbox"),
        ("<progress id=t></progress>", "progressbar"),
        ("<output id=t></output>", "status"),
        ("<meter id=t></meter>", "meter"),
        ("<fieldset id=t></fieldset>", "group"),
        ("<figure id=t></figure>", "figure"),
        (
            "<figure><figcaption id=t>c</figcaption></figure>",
            "caption",
        ),
        ("<table id=x><caption id=t>c</caption></table>", "caption"),
        ("<details id=t></details>", "group"),
        ("<hgroup id=t></hgroup>", "group"),
        ("<select><optgroup id=t></optgroup></select>", "group"),
        ("<select><option id=t>o</option></select>", "option"),
        ("<datalist id=t></datalist>", "listbox"),
        ("<time id=t>x</time>", "time"),
        ("<math id=t></math>", "math"),
        ("<svg id=t></svg>", "graphics-document"),
        ("<body id=t>x</body>", "generic"),
        ("<span id=t>x</span>", "generic"),
        ("<div id=t>x</div>", "generic"),
        ("<article id=t>x</article>", "article"),
        ("<main id=t>x</main>", "main"),
        ("<nav id=t>x</nav>", "navigation"),
        ("<search id=t>x</search>", "search"),
    ] {
        assert_eq!(r(html), Some(want), "{html}");
    }
}

mod conditions;
mod more;
